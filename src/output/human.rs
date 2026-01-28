// Human-readable formatting utilities

pub fn format_success(message: &str) {
    println!("✓ {}", message);
}

pub fn format_error(message: &str) {
    eprintln!("✗ {}", message);
}

pub fn format_warning(message: &str) {
    println!("⚠ {}", message);
}

pub fn format_info(message: &str) {
    println!("ℹ {}", message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formatting_functions() {
        // These functions just print, so we just verify they don't panic
        format_success("test");
        format_error("test");
        format_warning("test");
        format_info("test");
    }
}
