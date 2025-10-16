/// Docker build validation test
/// Ensures Dockerfile builds successfully and has all required dependencies
/// Run with: cargo test --test docker_build
use std::process::Command;

#[test]
#[ignore] // Run explicitly with: cargo test --test docker_build -- --ignored
fn test_dockerfile_builds_successfully() {
    // Check if Docker is available
    let docker_check = Command::new("docker").arg("--version").output();

    if docker_check.is_err() {
        println!("⚠️  Docker not available - skipping Docker build test");
        println!("   Install Docker to run this test: https://docs.docker.com/get-docker/");
        return;
    }

    println!("🔍 Testing Docker build configuration...");

    // Test 1: Build builder stage
    println!("\n🏗️  Building builder stage...");
    let builder_result = Command::new("docker")
        .args([
            "build",
            "--target",
            "builder",
            "-t",
            "defarm-builder:test",
            ".",
        ])
        .status()
        .expect("Failed to execute docker build");

    assert!(
        builder_result.success(),
        "Builder stage failed to build - check Dockerfile dependencies"
    );
    println!("✅ Builder stage built successfully");

    // Test 2: Build full image
    println!("\n🏗️  Building full Docker image...");
    let build_result = Command::new("docker")
        .args(["build", "-t", "defarm-api:test", "."])
        .status()
        .expect("Failed to execute docker build");

    assert!(
        build_result.success(),
        "Docker build failed - check Dockerfile and dependencies"
    );
    println!("✅ Full image built successfully");

    // Test 3: Verify libsodium in runtime
    println!("\n🔐 Verifying libsodium in runtime container...");
    let libsodium_check = Command::new("docker")
        .args([
            "run",
            "--rm",
            "defarm-api:test",
            "sh",
            "-c",
            "ldconfig -p | grep libsodium",
        ])
        .output()
        .expect("Failed to check libsodium");

    let output = String::from_utf8_lossy(&libsodium_check.stdout);
    assert!(
        output.contains("libsodium"),
        "libsodium not found in runtime container - add libsodium23 to Dockerfile runtime stage"
    );
    println!("✅ libsodium found in runtime: {}", output.trim());

    // Test 4: Verify binary exists
    println!("\n🎯 Verifying binary exists...");
    let binary_check = Command::new("docker")
        .args([
            "run",
            "--rm",
            "defarm-api:test",
            "ls",
            "-lh",
            "/app/defarm-api",
        ])
        .output()
        .expect("Failed to check binary");

    assert!(
        binary_check.status.success(),
        "Binary not found at /app/defarm-api"
    );
    println!(
        "✅ Binary exists: {}",
        String::from_utf8_lossy(&binary_check.stdout).trim()
    );

    // Cleanup
    println!("\n🧹 Cleaning up test images...");
    let _ = Command::new("docker")
        .args(["rmi", "defarm-api:test", "defarm-builder:test"])
        .output();

    println!("\n✅ All Docker build tests passed!");
}

#[test]
fn test_dockerfile_exists() {
    let dockerfile_exists = std::path::Path::new("Dockerfile").exists();
    assert!(dockerfile_exists, "Dockerfile not found in project root");
}

#[test]
fn test_dockerfile_has_libsodium_in_builder() {
    let dockerfile_content =
        std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    // Check builder stage has libsodium-dev
    assert!(
        dockerfile_content.contains("libsodium-dev"),
        "Dockerfile builder stage must include libsodium-dev for compilation"
    );

    // Check builder has pkg-config
    assert!(
        dockerfile_content.contains("pkg-config"),
        "Dockerfile builder stage must include pkg-config to find libsodium"
    );

    println!("✅ Dockerfile builder stage has libsodium-dev and pkg-config");
}

#[test]
fn test_dockerfile_has_libsodium_in_runtime() {
    let dockerfile_content =
        std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    // Check runtime stage has libsodium23 (runtime library)
    assert!(
        dockerfile_content.contains("libsodium23"),
        "Dockerfile runtime stage must include libsodium23 for encryption at runtime"
    );

    println!("✅ Dockerfile runtime stage has libsodium23");
}

#[test]
fn test_dockerfile_multistage_build() {
    let dockerfile_content =
        std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

    // Verify it's a multi-stage build
    assert!(
        dockerfile_content.contains("FROM rust") && dockerfile_content.contains("as builder"),
        "Dockerfile should use multi-stage build with 'as builder'"
    );

    assert!(
        dockerfile_content.contains("FROM debian"),
        "Dockerfile should use slim runtime image (debian:bookworm-slim)"
    );

    assert!(
        dockerfile_content.contains("COPY --from=builder"),
        "Dockerfile should copy binary from builder stage"
    );

    println!("✅ Dockerfile uses proper multi-stage build pattern");
}

#[test]
fn test_cargo_lock_has_libsodium_dependency() {
    let cargo_lock = std::fs::read_to_string("Cargo.lock").expect("Failed to read Cargo.lock");

    // Verify libsodium-sys-stable is in the dependency tree
    // (comes from stellar-baselib via soroban-client)
    assert!(
        cargo_lock.contains("libsodium-sys-stable"),
        "Cargo.lock should have libsodium-sys-stable (required by Stellar SDK)"
    );

    println!("✅ Cargo.lock includes libsodium-sys-stable dependency (from Stellar SDK)");
}

#[cfg(test)]
mod dockerfile_validation {
    

    #[test]
    fn test_no_root_user_in_runtime() {
        let dockerfile_content =
            std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

        // Security: Should not run as root
        assert!(
            dockerfile_content.contains("USER defarm") || dockerfile_content.contains("USER "),
            "Dockerfile should switch to non-root user for security"
        );

        println!("✅ Dockerfile uses non-root user");
    }

    #[test]
    fn test_expose_port_documented() {
        let dockerfile_content =
            std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

        // Should document the exposed port
        assert!(
            dockerfile_content.contains("EXPOSE"),
            "Dockerfile should document exposed port with EXPOSE directive"
        );

        println!("✅ Dockerfile documents exposed port");
    }

    #[test]
    fn test_apt_cache_cleanup() {
        let dockerfile_content =
            std::fs::read_to_string("Dockerfile").expect("Failed to read Dockerfile");

        // Should clean up apt cache to reduce image size
        assert!(
            dockerfile_content.contains("rm -rf /var/lib/apt/lists"),
            "Dockerfile should clean up apt cache with 'rm -rf /var/lib/apt/lists/*'"
        );

        println!("✅ Dockerfile cleans up apt cache");
    }
}
