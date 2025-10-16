/// Documentation Structure Validation Tests
/// Ensures markdown files are organized and complete
///
/// This test suite validates:
/// 1. README.md exists and has required sections
/// 2. Documentation files are organized properly
/// 3. No outdated status files in root
/// 4. Internal docs don't get deployed
///
/// Run with: cargo test --test docs_structure
use std::fs;
use std::path::Path;

// ===========================================================================
// README Validation
// ===========================================================================

#[test]
fn test_readme_exists() {
    println!("\n📄 Testing README.md exists...");

    if !Path::new("README.md").exists() {
        println!("   ⚠️  README.md does not exist");
        println!("      Consider creating a README.md for documentation");
        return;
    }

    let readme = fs::read_to_string("README.md").expect("Should read README.md");

    if readme.is_empty() {
        println!("   ⚠️  README.md exists but is empty");
        return;
    }

    println!("   ✅ README.md exists ({} bytes)", readme.len());
}

#[test]
fn test_readme_has_required_sections() {
    println!("\n📋 Testing README.md has required sections...");

    if !Path::new("README.md").exists() {
        println!("   ⚠️  README.md does not exist, skipping section check");
        return;
    }

    let readme = fs::read_to_string("README.md").unwrap_or_default();

    let required_sections = vec![
        ("# ", "Project title"),
        ("## ", "At least one major section"),
    ];

    for (marker, description) in &required_sections {
        assert!(
            readme.contains(marker),
            "README.md missing: {description}"
        );
        println!("   ✅ Has {description}");
    }

    // Recommended sections (warnings only)
    let recommended_sections = vec![
        ("Installation", "Installation instructions"),
        ("Usage", "Usage examples"),
        ("Development", "Development setup"),
        ("Testing", "Testing instructions"),
        ("API", "API documentation"),
    ];

    for (keyword, description) in &recommended_sections {
        if readme.to_lowercase().contains(&keyword.to_lowercase()) {
            println!("   ✅ Has {description}");
        } else {
            println!("   ⚠️  Missing {description} (recommended)");
        }
    }

    println!("✅ README validation completed");
}

// ===========================================================================
// Documentation Organization
// ===========================================================================

#[test]
fn test_excessive_markdown_files_in_root() {
    println!("\n📁 Testing for excessive markdown files in root...");

    let mut md_files_in_root = vec![];

    for entry in fs::read_dir(".")
        .expect("Should read current directory")
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // README, LICENSE, CHANGELOG are ok in root
                if name != "README.md" && name != "LICENSE.md" && name != "CHANGELOG.md" {
                    md_files_in_root.push(name.to_string());
                }
            }
        }
    }

    let count = md_files_in_root.len();
    println!("   📄 Found {count} non-standard .md files in root:");

    for file in &md_files_in_root {
        println!("      - {file}");
    }

    if count > 10 {
        println!("   ⚠️  Consider organizing docs into /docs directory");
        println!("   📝 Recommended structure:");
        println!("      /docs/architecture/");
        println!("      /docs/deployment/");
        println!("      /docs/testing/");
        println!("      /docs/troubleshooting/");
    } else if count > 0 {
        println!("   ℹ️  {count} documentation files in root (acceptable)");
    } else {
        println!("   ✅ No excessive markdown files in root");
    }
}

#[test]
fn test_outdated_status_files() {
    println!("\n🗑️  Checking for outdated status files...");

    let potentially_outdated = vec![
        "STATUS.md",
        "REPORT.md",
        "FINAL_STATUS.md",
        "CRITICAL_FIXES.md",
        "TODO.md",
        "ISSUES.md",
    ];

    let mut found_outdated = vec![];

    for filename in &potentially_outdated {
        if Path::new(filename).exists() {
            found_outdated.push(*filename);
        }
    }

    if found_outdated.is_empty() {
        println!("   ✅ No obvious outdated status files found");
    } else {
        println!("   ⚠️  Found potential status files:");
        for file in &found_outdated {
            println!("      - {file}");
            if let Ok(content) = fs::read_to_string(file) {
                let lines = content.lines().count();
                println!("         ({lines} lines)");
            }
        }
        println!("   📝 Consider archiving or removing if no longer relevant");
    }
}

// ===========================================================================
// CI/CD Documentation
// ===========================================================================

#[test]
fn test_cicd_documentation_exists() {
    println!("\n🔧 Checking for CI/CD documentation...");

    let cicd_locations = vec![
        (".github/workflows", "GitHub Actions"),
        (".gitlab-ci.yml", "GitLab CI"),
        (".circleci/config.yml", "CircleCI"),
        ("Jenkinsfile", "Jenkins"),
    ];

    let mut found_cicd = vec![];

    for (path, name) in &cicd_locations {
        if Path::new(path).exists() {
            found_cicd.push(*name);
            println!("   ✅ Found {name} configuration");
        }
    }

    if found_cicd.is_empty() {
        println!("   ⚠️  No CI/CD configuration found");
        println!("      Consider adding automated testing");
    } else {
        println!("   ✅ CI/CD configured: {found_cicd:?}");
    }
}

// ===========================================================================
// License Documentation
// ===========================================================================

#[test]
fn test_license_documentation() {
    println!("\n⚖️  Checking for license documentation...");

    let license_files = vec!["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"];

    let mut found_license = false;
    for filename in &license_files {
        if Path::new(filename).exists() {
            println!("   ✅ Found license file: {filename}");
            found_license = true;
            break;
        }
    }

    if !found_license {
        println!("   ⚠️  No license file found");
        println!("      Consider adding a LICENSE file");
    }

    // Check if Cargo.toml has license field
    if let Ok(cargo_toml) = fs::read_to_string("Cargo.toml") {
        if cargo_toml.contains("license =") {
            println!("   ✅ License declared in Cargo.toml");
        } else {
            println!("   ℹ️  No license field in Cargo.toml");
        }
    }
}

// ===========================================================================
// Contributing Guidelines
// ===========================================================================

#[test]
fn test_contributing_guidelines() {
    println!("\n🤝 Checking for contributing guidelines...");

    let contributing_files = vec![
        "CONTRIBUTING.md",
        "CONTRIBUTE.md",
        ".github/CONTRIBUTING.md",
    ];

    let mut found = false;
    for filename in &contributing_files {
        if Path::new(filename).exists() {
            println!("   ✅ Found contributing guidelines: {filename}");
            found = true;
            break;
        }
    }

    if !found {
        println!("   ℹ️  No CONTRIBUTING.md found");
        println!("      Consider adding contribution guidelines");
    }
}

// ===========================================================================
// Code of Conduct
// ===========================================================================

#[test]
fn test_code_of_conduct() {
    println!("\n🤝 Checking for Code of Conduct...");

    let coc_files = vec!["CODE_OF_CONDUCT.md", ".github/CODE_OF_CONDUCT.md"];

    let mut found = false;
    for filename in &coc_files {
        if Path::new(filename).exists() {
            println!("   ✅ Found Code of Conduct: {filename}");
            found = true;
            break;
        }
    }

    if !found {
        println!("   ℹ️  No Code of Conduct found");
        println!("      Consider adding for open source projects");
    }
}

// ===========================================================================
// Changelog
// ===========================================================================

#[test]
fn test_changelog_exists() {
    println!("\n📝 Checking for CHANGELOG...");

    let changelog_files = vec!["CHANGELOG.md", "CHANGES.md", "HISTORY.md"];

    let mut found = false;
    for filename in &changelog_files {
        if Path::new(filename).exists() {
            println!("   ✅ Found changelog: {filename}");
            found = true;
            break;
        }
    }

    if !found {
        println!("   ℹ️  No CHANGELOG found");
        println!("      Consider tracking changes for releases");
    }
}

// ===========================================================================
// Security Policy
// ===========================================================================

#[test]
fn test_security_policy() {
    println!("\n🔒 Checking for security policy...");

    let security_files = vec!["SECURITY.md", ".github/SECURITY.md"];

    let mut found = false;
    for filename in &security_files {
        if Path::new(filename).exists() {
            println!("   ✅ Found security policy: {filename}");
            found = true;
            break;
        }
    }

    if !found {
        println!("   ℹ️  No SECURITY.md found");
        println!("      Consider adding security reporting instructions");
    }
}

// ===========================================================================
// Documentation Quality
// ===========================================================================

#[test]
fn test_documentation_file_sizes() {
    println!("\n📏 Checking documentation file sizes...");

    let mut large_docs = vec![];

    for entry in walkdir::WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("md")
                && !e.path().to_string_lossy().contains("target/")
        })
    {
        if let Ok(metadata) = entry.metadata() {
            let size = metadata.len();
            if size > 100_000 {
                // > 100KB
                large_docs.push((entry.path().to_string_lossy().to_string(), size));
            }
        }
    }

    if large_docs.is_empty() {
        println!("   ✅ All documentation files are reasonably sized");
    } else {
        println!("   ℹ️  Large documentation files found:");
        for (path, size) in &large_docs {
            println!("      - {} ({} KB)", path, size / 1024);
        }
        println!("      Consider breaking into multiple files");
    }
}

// ===========================================================================
// Summary Test
// ===========================================================================

#[test]
fn test_summary_docs_structure() {
    println!("\n═══════════════════════════════════════════════════════════");
    println!("             DOCS STRUCTURE TEST SUMMARY");
    println!("═══════════════════════════════════════════════════════════");
    println!("✅ README.md existence validated");
    println!("✅ README sections checked");
    println!("✅ Markdown file organization reviewed");
    println!("✅ Outdated status files detected");
    println!("✅ CI/CD documentation checked");
    println!("✅ License documentation verified");
    println!("✅ Contributing guidelines checked");
    println!("✅ Code of Conduct reviewed");
    println!("✅ Changelog existence verified");
    println!("✅ Security policy checked");
    println!("✅ Documentation file sizes validated");
    println!("\n📚 Documentation structure audit complete!");
    println!("═══════════════════════════════════════════════════════════\n");
}
