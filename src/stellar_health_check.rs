/// Stellar CLI Health Check
/// Verifies that Stellar CLI is properly configured at startup

pub async fn check_stellar_cli_configuration() {
    println!("🔍 Checking Stellar CLI configuration...");

    // Check if stellar CLI is available
    match tokio::process::Command::new("stellar")
        .args(&["network", "ls"])
        .output()
        .await
    {
        Ok(output) => {
            let networks = String::from_utf8_lossy(&output.stdout);
            let networks_str = networks.trim();

            let has_testnet = networks_str.contains("testnet");
            let has_mainnet = networks_str.contains("mainnet");

            if has_testnet && has_mainnet {
                println!("   ✅ Stellar CLI configured (testnet + mainnet)");
            } else if has_testnet {
                println!("   ✅ Stellar CLI configured (testnet only)");
                println!("   ⚠️  Mainnet not configured - mainnet adapter will not work");
                println!("   💡 Run: stellar network add mainnet --rpc-url https://soroban-rpc.mainnet.stellar.org --network-passphrase \"Public Global Stellar Network ; September 2015\"");
            } else if has_mainnet {
                println!("   ⚠️  Testnet not configured - testnet adapter will not work");
                println!("   💡 Run: stellar network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase \"Test SDF Network ; September 2015\"");
            } else {
                println!("   ⚠️  No Stellar networks configured");
                println!("   💡 Configure networks:");
                println!("      stellar network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase \"Test SDF Network ; September 2015\"");
                println!("      stellar network add mainnet --rpc-url https://soroban-rpc.mainnet.stellar.org --network-passphrase \"Public Global Stellar Network ; September 2015\"");
            }
        },
        Err(e) => {
            println!("   ❌ Stellar CLI not found or not in PATH");
            println!("   💡 Install: cargo install --locked stellar-cli");
            println!("   Error: {}", e);
        }
    }

    println!();
}
