use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

/// Cached tokenizer instance (initialized once)
fn get_tokenizer() -> Result<&'static CoreBPE, String> {
    static TOKENIZER: OnceLock<Result<CoreBPE, String>> = OnceLock::new();
    TOKENIZER
        .get_or_init(|| match tiktoken_rs::cl100k_base() {
            Ok(bpe) => Ok(bpe),
            Err(e) => Err(format!("Failed to initialize tiktoken tokenizer: {}", e)),
        })
        .as_ref()
        .map_err(|e| e.clone())
}

/// Count tokens in a single text string using cl100k_base (GPT-4/GPT-3.5)
pub fn count_tokens(text: &str) -> Result<usize, String> {
    let tokenizer = get_tokenizer()?;
    Ok(tokenizer.encode_with_special_tokens(text).len())
}

/// Count tokens for a batch of messages (sum of all)
pub fn count_tokens_batch(messages: &[String]) -> Result<usize, String> {
    let tokenizer = get_tokenizer()?;
    let mut total = 0usize;
    for msg in messages {
        total += tokenizer.encode_with_special_tokens(msg).len();
    }
    Ok(total)
}
