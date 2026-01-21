// Text replacement macros for default values.

use crate::error::Result;
use anyhow::anyhow;

pub struct MacroEngine {
    cfg: Option<rustc_cfg::Cfg>,
    target_triple: Option<String>,
}

impl MacroEngine {
    pub fn new() -> Self {
        Self {
            cfg: None,
            target_triple: None,
        }
    }

    pub fn with_target_triple(mut self, triple: String) -> Result<Self> {
        if let Ok(cfg) = rustc_cfg::Cfg::of(&triple) {
            self.target_triple = Some(triple);
            self.cfg = Some(cfg);
            return Ok(self);
        } else {
            return Err(anyhow!("invalid target triple '{triple}'").into());
        }
    }

    fn replace_expr(&self, expr: &str) -> Option<String> {
        match expr {
            "target_triple" => self.target_triple.clone(),
            "target_arch" => self.cfg.as_ref().map(|cfg| cfg.target_arch.clone()),
            _ => None,
        }
    }

    pub fn exec(&self, input: &str) -> Result<String> {
        let mut output = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' && chars.peek() == Some(&'{') {
                chars.next();
                let mut expr = String::new();
                while let Some(&next_char) = chars.peek() {
                    if next_char == '}' {
                        chars.next();
                        break;
                    } else {
                        expr.push(next_char);
                        chars.next();
                    }
                }
                if let Some(replacement) = self.replace_expr(&expr) {
                    output.push_str(&replacement);
                } else {
                    return Err(anyhow!("unknown macro expression '{expr}'").into());
                }
            } else {
                output.push(c);
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_plain_text() {
        let engine = MacroEngine::new();
        let result = engine.exec("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn exec_with_target_triple_macro() {
        let engine = MacroEngine::new()
            .with_target_triple("x86_64-unknown-linux-gnu".to_string())
            .unwrap();
        let result = engine.exec("Target: ${target_triple}").unwrap();
        assert_eq!(result, "Target: x86_64-unknown-linux-gnu");
    }

    #[test]
    fn exec_with_target_arch_macro() {
        let engine = MacroEngine::new()
            .with_target_triple("x86_64-unknown-linux-gnu".to_string())
            .unwrap();
        let result = engine.exec("Arch: ${target_arch}").unwrap();
        assert_eq!(result, "Arch: x86_64");
    }

    #[test]
    fn exec_multiple_macros() {
        let engine = MacroEngine::new()
            .with_target_triple("aarch64-unknown-linux-gnu".to_string())
            .unwrap();
        let result = engine.exec("${target_triple} on ${target_arch}").unwrap();
        assert_eq!(result, "aarch64-unknown-linux-gnu on aarch64");
    }

    #[test]
    fn exec_unknown_macro_fails() {
        let engine = MacroEngine::new();
        let result = engine.exec("Value: ${unknown_macro}");
        assert!(result.is_err());
    }

    #[test]
    fn exec_invalid_target_triple() {
        let result = MacroEngine::new().with_target_triple("invalid-triple".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn exec_no_macros_in_string() {
        let engine = MacroEngine::new();
        let result = engine.exec("plain text without macros").unwrap();
        assert_eq!(result, "plain text without macros");
    }

    #[test]
    fn exec_dollar_without_brace() {
        let engine = MacroEngine::new();
        let result = engine.exec("price $100").unwrap();
        assert_eq!(result, "price $100");
    }

    #[test]
    fn exec_empty_macro() {
        let engine = MacroEngine::new();
        let result = engine.exec("value: ${}");
        assert!(result.is_err());
    }

    #[test]
    fn exec_macro_without_target_triple_set() {
        let engine = MacroEngine::new();
        let result = engine.exec("${target_triple}");
        assert!(result.is_err());
    }
}
