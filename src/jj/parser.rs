use crate::error::Result;

pub struct JjStatus {
    pub working_copy: Option<String>,
}

pub fn parse_status(output: &str) -> Result<JjStatus> {
    // Simple parser for jj status output
    // Extract working copy location if present
    let working_copy = output
        .lines()
        .find(|line| line.contains("Working copy"))
        .and_then(|line| line.split(':').nth(1).map(|s| s.trim().to_string()));

    Ok(JjStatus { working_copy })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_status_basic() {
        let output = "Working copy: main @ abc123\nParent commit: xyz789";
        let status = parse_status(output).unwrap();
        assert!(status.working_copy.is_some());
    }

    #[test]
    fn test_parse_status_empty() {
        let output = "No working copy";
        let status = parse_status(output).unwrap();
        assert!(status.working_copy.is_none());
    }
}
