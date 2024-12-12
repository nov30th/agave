use std::str::FromStr;
use std::net::UdpSocket;
use std::sync::mpsc::{self, Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
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

fn analyze_transaction(data: &[u8]) -> Option<()> {
    let tx: VersionedTransaction = bincode::deserialize(data).ok()?;

    let signature = tx.signatures.first()?;
    println!("Transaction signature: {}", signature);

    match &tx.message {
        VersionedMessage::Legacy(message) => {
            analyze_message_accounts(message.account_keys.as_slice(), message.instructions.as_slice(), signature)
        }
        VersionedMessage::V0(message) => {
            analyze_message_accounts(message.account_keys.as_slice(), message.instructions.as_slice(), signature)
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

    let raydium_v4 = Pubkey::from_str(RAYDIUM_V4_PROGRAM_ID).ok()?;
    let raydium_swap = Pubkey::from_str(RAYDIUM_SWAP_PROGRAM).ok()?;

    for (i, ix) in instructions.iter().enumerate() {
        let program_id = account_keys[ix.program_id_index as usize];
        println!("\nInstruction {} Program ID: {}", i, program_id);

        if program_id == raydium_v4 || program_id == raydium_swap {
            println!("Found Raydium transaction! Signature: {}", signature);
            println!("Instruction accounts:");
            for account_idx in ix.accounts.iter() {
                println!("  {}", account_keys[*account_idx as usize]);
            }

            if let Some((amount, token_mint)) = parse_raydium_instruction(&ix.data) {
                println!("Token amount: {}", amount);
                println!("Token mint: {}", token_mint);
                return Some(());
            }
        }
    }
    None
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