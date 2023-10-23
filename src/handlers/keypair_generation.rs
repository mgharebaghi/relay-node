pub mod generation {
    use bip39::Mnemonic;
    use log::error;
    use seed15::random_seed;
    use sp_core::{ecdsa, Pair};

    pub fn keys_generate(answer: String, wallet: &mut String) {
        if answer == "n".to_string() || answer == "N".to_string() {
            let seed = random_seed();
            let mnomenic = Mnemonic::from_entropy(&seed).unwrap();
            let phrases = &mnomenic.to_string();
            let keys = ecdsa::Pair::from_phrase(&phrases, None).unwrap();
            println!("your phrases key:\n{}\n-----------------", phrases);
            wallet.push_str(&keys.0.public().to_string());
            println!("your wallet address:\n{}\n----------------", wallet);
        } else {
            if let Ok(keys) = ecdsa::Pair::from_phrase(&answer, None) {
                wallet.push_str(&keys.0.public().to_string());
                println!("your wallet address:\n{}\n----------------", wallet);
            } else {
                error!("your phrases key is wrong, please enter a correct!");
                wallet.push_str(&"emptey".to_string());
            }
        }
    }
}
