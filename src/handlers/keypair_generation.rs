pub mod generation {
    use std::io::stdout;

    use bip39::Mnemonic;
    use crossterm::{
        execute,
        style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
    };
    use seed15::random_seed;
    use sp_core::{ecdsa, Pair};

    pub fn keys_generate(answer: String, wallet: &mut String) {
        if answer == "n".to_string() || answer == "N".to_string() {
            let seed = random_seed();
            let mnomenic = Mnemonic::from_entropy(&seed).unwrap();
            let phrases = &mnomenic.to_string();
            let keys = ecdsa::Pair::from_phrase(&phrases, None).unwrap();

            execute!(
                stdout(),
                SetForegroundColor(Color::Green),
                Print("Your private key:".bold()),
                ResetColor
            )
            .unwrap();
            println!("\n{}", phrases);

            wallet.push_str(&keys.0.public().to_string());
            execute!(
                stdout(),
                SetForegroundColor(Color::Green),
                Print("Your wallet address:".bold()),
                ResetColor
            )
            .unwrap();
            println!("\n{}", wallet);

            execute!(
                stdout(),
                SetForegroundColor(Color::DarkYellow),
                Print("Please keep Your private key and wallet address in a safe where and then enter Y to continue:\n".bold()),
                ResetColor
            )
            .unwrap();

            loop {
                let mut save_answ = String::new();
                std::io::stdin().read_line(&mut save_answ).unwrap();
                if save_answ.trim() == "y" || save_answ.trim() == "Y" {
                    break;
                } else {
                    execute!(
                        stdout(),
                        SetForegroundColor(Color::Red),
                        Print(
                            "If you save Your adresses please enter Y:\n"
                                .bold()
                        ),
                        ResetColor
                    )
                    .unwrap();
                }
            }
        } else {
            if let Ok(keys) = ecdsa::Pair::from_phrase(&answer, None) {
                wallet.push_str(&keys.0.public().to_string());
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Green),
                    Print("Your wallet address:".bold()),
                    ResetColor
                )
                .unwrap();
                println!("\n{}", wallet);
            } else {
                execute!(
                    stdout(),
                    SetForegroundColor(Color::Red),
                    Print("your phrases key is wrong, please enter a correct!\n".bold()),
                    ResetColor
                )
                .unwrap();
                wallet.push_str(&"emptey".to_string());
            }
        }
    }
}
