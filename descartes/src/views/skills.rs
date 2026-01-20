//! Skills display functions

use crate::Result;

/// Show list of available skills
pub fn show_skills_list() -> Result<()> {
    let skills_dir = std::path::PathBuf::from(".descartes/skills");

    if !skills_dir.exists() {
        println!("No skills directory found.");
        println!("Run 'descartes skills init' to create default skills.");
        return Ok(());
    }

    println!("Available Skills");
    println!("================\n");

    let mut found = false;
    for entry in std::fs::read_dir(&skills_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "md") {
            let name = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Try to extract description from first line
            let description = std::fs::read_to_string(&path)
                .ok()
                .and_then(|content| {
                    content
                        .lines()
                        .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                        .map(|s| s.trim().to_string())
                })
                .unwrap_or_default();

            println!("  {} - {}", name, description);
            found = true;
        }
    }

    if !found {
        println!("No skills found in {:?}", skills_dir);
    }

    Ok(())
}

/// Show a specific skill's content
pub fn show_skill_detail(name: &str) -> Result<()> {
    let skill_file = std::path::PathBuf::from(format!(".descartes/skills/{}.md", name));

    if !skill_file.exists() {
        println!("Skill not found: {}", name);
        return Ok(());
    }

    let content = std::fs::read_to_string(&skill_file)?;
    println!("{}", content);

    Ok(())
}
