use std::str::FromStr;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use chrono::Utc;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::transaction::VersionedTransaction;

// Raydium DEX program IDs
const RAYDIUM_V4_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const RAYDIUM_SWAP_PROGRAM: &str = "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv";

struct UdpClient {
    socket: UdpSocket,
    sender: Sender<Vec<u8>>,
}

impl UdpClient {
    fn new(addr: &str) -> Result<(Self, Arc<Mutex<Receiver<Vec<u8>>>>), std::io::Error> {
        let socket = UdpSocket::bind(addr)?;
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        Ok((UdpClient { socket, sender }, receiver))
    }

    fn start_receiving(&self) {
        let mut buf = [0; 1024 * 64];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, _)) => {
                    let data = buf[..size].to_vec();
                    if let Err(e) = self.sender.send(data) {
                        eprintln!("Error sending to channel: {}", e);
                        break;
                    }
                }
                Err(e) => eprintln!("Error receiving data: {}", e),
            }
        }
    }
}

fn analyze_swap_accounts_and_inner_instructions(
    account_keys: &[Pubkey],
    instructions: &[CompiledInstruction],
    signature: &Signature,
) -> Option<()> {
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").ok()?;
    let raydium_v4 = Pubkey::from_str(RAYDIUM_V4_PROGRAM_ID).ok()?;

    for (i, ix) in instructions.iter().enumerate() {
        let program_id = account_keys[ix.program_id_index as usize];

        if program_id == raydium_v4 {
            println!("\nRaydium Swap Transaction Found!");
            println!("Signature: {}", signature);

            // 获取关键账户
            for (idx, account_idx) in ix.accounts.iter().enumerate() {
                let account = &account_keys[*account_idx as usize];
                match idx {
                    // Token Program accounts
                    15 => println!("Source Token Account: {} (User's Token Account)", account),
                    16 => println!("Destination Token Account: {} (User's Token Account)", account),
                    5 => println!("Pool Token Account 1: {} (AMM Token Account)", account),
                    6 => println!("Pool Token Account 2: {} (AMM Token Account)", account),
                    _ => {}
                }
            }

            // 解析指令数据
            if ix.data.len() >= 17 {
                let amount_in = {
                    let mut amount_bytes = [0u8; 8];
                    amount_bytes.copy_from_slice(&ix.data[1..9]);
                    u64::from_le_bytes(amount_bytes)
                };
0
                println!("\nSwap Amount Details:");
                // 对于SOL，需要除以1e9；对于其他代币，需要根据小数位数调整
                println!("Amount In: {} (raw value: {})",
                         amount_in as f64 / 1_000_000_000.0,
                         amount_in);

                // 我们还需要获取代币账户的mint地址
                // 这需要调用RPC来获取账户信息
                println!("\nNote: To get token mint addresses, we need to query the token accounts:");
                println!("User Source Token Account: {}", account_keys[ix.accounts[15] as usize]);
                println!("User Destination Token Account: {}", account_keys[ix.accounts[16] as usize]);

                //println current time use std lib
                let system_time = Utc::now();
                println!("系统时间: {}", system_time.format("%Y年%m月%d日 %H时%M分%S秒"));                

            }

            return Some(());
        }
    }
    None
}

fn analyze_transaction(data: &[u8]) -> Option<()> {
    let tx: VersionedTransaction = bincode::deserialize(data).ok()?;

    let signature = tx.signatures.first()?;
    println!("Transaction signature: {}", signature);

    // 解析内部指令
    match &tx.message {
        VersionedMessage::Legacy(message) => {
            analyze_swap_accounts_and_inner_instructions(message.account_keys.as_slice(),
                                                         message.instructions.as_slice(),
                                                         signature)
        }
        VersionedMessage::V0(message) => {
            analyze_swap_accounts_and_inner_instructions(message.account_keys.as_slice(),
                                                         message.instructions.as_slice(),
                                                         signature)
        }
    }
}


fn analyze_message_accounts(
    account_keys: &[Pubkey],
    instructions: &[CompiledInstruction],
    signature: &Signature,
) -> Option<()> {
    println!("\nAccount addresses:");
    for (i, key) in account_keys.iter().enumerate() {
        println!("Account {}: {}", i, key);
    }

    // Raydium 和其他重要合约地址
    let raydium_v4 = Pubkey::from_str(RAYDIUM_V4_PROGRAM_ID).ok()?;
    let raydium_swap = Pubkey::from_str(RAYDIUM_SWAP_PROGRAM).ok()?;
    let token_program = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

    for (i, ix) in instructions.iter().enumerate() {
        let program_id = account_keys[ix.program_id_index as usize];
        println!("\nInstruction {} Program ID: {}", i, program_id);

        if program_id == raydium_v4 || program_id == raydium_swap {
            println!("Found Raydium transaction! Signature: {}", signature);
            println!("\nSwap Account Details:");

            // 解析关键账户
            for (idx, account_idx) in ix.accounts.iter().enumerate() {
                let account = &account_keys[*account_idx as usize];
                match idx {
                    0 => println!("Token Program: {}", account),
                    1 => println!("AMM Account: {}", account),
                    2 => println!("AMM Authority: {}", account),
                    5 => println!("Pool Token Account 1: {}", account),
                    6 => println!("Pool Token Account 2: {}", account),
                    15 => println!("User Source Token Account: {}", account),
                    16 => println!("User Destination Token Account: {}", account),
                    17 => println!("User Authority: {}", account),
                    _ => println!("Account {}: {}", idx, account),
                }
            }

            // 解析程序日志
            if let Some(ray_log) = find_ray_log(&ix.data) {
                println!("\nRaydium Log Data:");
                println!("{}", ray_log);
            }

            // 打印完整的指令数据（十六进制）
            println!("\nInstruction data (hex):");
            for (i, chunk) in ix.data.chunks(32).enumerate() {
                let hex_string: String = chunk.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();
                println!("{:04x}: {}", i * 32, hex_string);
            }

            // 解析 Raydium 指令数据
            if ix.data.len() >= 17 {
                let discriminator = ix.data[0];
                let amount_in = {
                    let mut amount_bytes = [0u8; 8];
                    amount_bytes.copy_from_slice(&ix.data[1..9]);
                    u64::from_le_bytes(amount_bytes)
                };

                let min_amount_out = {
                    let mut amount_bytes = [0u8; 8];
                    amount_bytes.copy_from_slice(&ix.data[9..17]);
                    u64::from_le_bytes(amount_bytes)
                };

                println!("\nParsed Swap Details:");
                println!("Discriminator: {}", discriminator);
                println!("Amount In: {} lamports", amount_in);
                println!("Minimum Amount Out: {} tokens", min_amount_out);
            }

            return Some(());
        }
    }
    None
}

fn find_ray_log(data: &[u8]) -> Option<String> {
    // Base64 解码处理
    if data.len() > 8 {
        // 这里需要具体实现，从程序日志中解析出ray_log的内容
        None
    } else {
        None
    }
}


fn parse_raydium_instruction(data: &[u8]) -> Option<(u64, Pubkey)> {
    if data.len() < 9 {
        return None;
    }

    let amount = u64::from_le_bytes(data[0..8].try_into().ok()?);
    let mut pubkey_bytes = [0u8; 32];
    pubkey_bytes.copy_from_slice(&data[8..40]);
    let token_mint = Pubkey::new_from_array(pubkey_bytes);

    Some((amount, token_mint))
}

fn main() {
    let (client, receiver) = UdpClient::new("127.0.0.1:44444").unwrap();

    let receiver_thread = thread::spawn(move || {
        client.start_receiving();
    });

    let consumer_thread = thread::spawn(move || {
        while let Ok(rx) = receiver.lock() {
            if let Ok(data) = rx.recv() {
                if let Some(()) = analyze_transaction(&data) {
                    println!("Found target transaction, exiting...");
                    std::process::exit(0);
                }
            }
        }
    });

    receiver_thread.join().unwrap();
    consumer_thread.join().unwrap();
}