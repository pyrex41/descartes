//! Transcript display functions

use crate::{Config, Result, transcript};

/// Show list of transcripts
pub fn show_transcripts(config: &Config, today: bool, session: Option<String>) -> Result<()> {
    let transcripts = transcript::list_transcripts(config, today, session)?;

    if transcripts.is_empty() {
        println!("No transcripts found");
        return Ok(());
    }

    println!("Transcripts");
    println!("===========\n");

    for (idx, name) in transcripts.iter().enumerate() {
        println!("{:>4}. {}", idx + 1, name);
    }

    Ok(())
}

/// Show a single transcript
pub fn show_transcript(config: &Config, session_id: &str) -> Result<()> {
    let transcript = transcript::load(config, session_id)?;

    // Print the transcript in SCG format
    println!("{}", transcript.to_scg());

    Ok(())
}
