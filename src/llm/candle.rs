use anyhow::Result;

use hf_hub::{api::sync::Api, Repo, RepoType};

use crate::files::File;
use crate::{EmbeddingsResult, LLMBackend, PromptBackend};

use fastembed::TextEmbedding;
use futures::stream::StreamExt;

use crate::Conversation;
use candle_core::Tensor;
use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use candle_transformers::generation::{LogitsProcessor, Sampling};
use candle_transformers::models::llama::{Cache, Config, Llama, LlamaConfig};
use tokenizers::Tokenizer;

pub struct CandleBackend {
    embed_model: TextEmbedding,
    prompt_model: Llama,
    tokenizer: Tokenizer,
    cache: Cache,
    config: Config,
}

#[async_trait::async_trait]
impl LLMBackend for CandleBackend {
    fn new() -> Result<Self> {
        let device = Device::Cpu;
        let dtype = DType::F32;
        let (llama, tokenizer_filename, cache, config) = {
            let api = Api::new()?;
            let model_id = "meta-llama/Meta-Llama-3-8B-Instruct".to_string();

            println!("loading the model weights from {model_id}");
            let revision = "main".to_string();
            let api = api.repo(Repo::with_revision(model_id, RepoType::Model, revision));

            let tokenizer_filename = api.get("tokenizer.json")?;
            let config_filename = api.get("config.json")?;
            let config: LlamaConfig = serde_json::from_slice(&std::fs::read(config_filename)?)?;
            let config = config.into_config(false);

            let filenames = hub_load_safetensors(&api, "model.safetensors.index.json")?;

            let cache = Cache::new(false, dtype, &config, &device)?;

            let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };
            (Llama::load(vb, &config)?, tokenizer_filename, cache, config)
        };
        let tokenizer = Tokenizer::from_file(tokenizer_filename)
            .map_err(|e| anyhow::anyhow!("Couldn't create tokenizer: {e}"))?;
        Ok(Self {
            embed_model: TextEmbedding::try_new(Default::default())?,
            prompt_model: llama,
            tokenizer,
            cache,
            config,
        })
    }

    async fn embed_file(&self, file: &File) -> Result<EmbeddingsResult> {
        let sentence_as_vec_str: Vec<&str> = file.sentences.iter().map(|s| s.as_str()).collect();
        println!("Embedding: {:?}", file.path);
        let embeddings = self
            .embed_model
            .embed(sentence_as_vec_str, None)
            .inspect_err(|x| println!("Failed to embed: {x:?}"))?;

        Ok(EmbeddingsResult::CandleEmbeddings(embeddings))
    }

    async fn embed_sentence(&self, prompt: &str) -> Result<EmbeddingsResult> {
        let embedding = self
            .embed_model
            .embed(vec![prompt], None)
            .inspect_err(|x| println!("Failed to embed: {x:?}"))?;

        Ok(EmbeddingsResult::CandleEmbedding(embedding))
    }
}

const EOS_TOKEN: &str = "</s>";

//#[async_trait::async_trait]
//impl PromptBackend for CandleBackend {
//    async fn chat_stream(&self, prompt: &str, contents: &str) -> Result<Conversation> {
//        let content = format!("{}\n Context: {}\n Be concise", prompt, contents);
//        let tokenizer = self.tokenizer;
//
//        let eos_token_id = tokenizer.token_to_id(EOS_TOKEN);
//
//        let mut tokens = tokenizer
//            .encode(content, true)
//            .map_err(|e| anyhow::anyhow!("Failed to encode content: {e}"))?
//            .get_ids()
//            .to_vec();
//
//        let mut tokenizer = TokenOutputStream::new(tokenizer);
//        println!("starting the inference loop");
//        print!("{prompt}");
//        let mut logits_processor = {
//            let temperature = 0.;
//            let sampling = Sampling::All { temperature };
//
//            LogitsProcessor::from_sampling(SEED_RNG, sampling)
//        };
//
//        let mut index_pos = 0;
//        let mut token_generated = 0;
//        let sample_len = 200;
//
//        let meme = futures::stream::unfold(0, |state: usize| async move {
//            if state >= sample_len {
//                return None;
//            }
//
//            let (context_size, context_index) = (tokens.len(), 0);
//            let ctxt = &tokens[tokens.len().saturating_sub(context_size)..];
//            let input = Tensor::new(ctxt, &Device::Cpu)
//                .unwrap()
//                .unsqueeze(0)
//                .unwrap();
//            let logits = self
//                .prompt_model
//                .forward(&input, context_index, &mut self.cache)
//                .unwrap();
//            let logits = logits.squeeze(0).unwrap();
//            let repeat_penalty = 1.1;
//            let repeat_last_n = 50;
//            let logits = {
//                let start_at = tokens.len().saturating_sub(repeat_last_n);
//                candle_transformers::utils::apply_repeat_penalty(
//                    &logits,
//                    repeat_penalty,
//                    &tokens[start_at..],
//                )
//                .unwrap()
//            };
//            index_pos += ctxt.len();
//
//            let next_token = logits_processor.sample(&logits)?;
//            token_generated += 1;
//            tokens.push(next_token);
//
//            if Some(next_token) == eos_token_id {
//                return None;
//            }
//            if let Some(t) = tokenizer.next_token(next_token).unwrap() {
//                let next_state = state += 1;
//                Some((t, next_state))
//            } else {
//                None
//            }
//        });
//
//        Ok(meme)
//    }
//}

/// This is a wrapper around a tokenizer to ensure that tokens can be returned to the user in a
/// streaming way rather than having to wait for the full decoding.
pub struct TokenOutputStream {
    tokenizer: tokenizers::Tokenizer,
    tokens: Vec<u32>,
    prev_index: usize,
    current_index: usize,
}

impl TokenOutputStream {
    pub fn new(tokenizer: tokenizers::Tokenizer) -> Self {
        Self {
            tokenizer,
            tokens: Vec::new(),
            prev_index: 0,
            current_index: 0,
        }
    }

    pub fn into_inner(self) -> tokenizers::Tokenizer {
        self.tokenizer
    }

    fn decode(&self, tokens: &[u32]) -> Result<String> {
        match self.tokenizer.decode(tokens, true) {
            Ok(str) => Ok(str),
            Err(err) => Err(anyhow::anyhow!("cannot decode: {err}")),
        }
    }

    // https://github.com/huggingface/text-generation-inference/blob/5ba53d44a18983a4de32d122f4cb46f4a17d9ef6/server/text_generation_server/models/model.py#L68
    pub fn next_token(&mut self, token: u32) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        self.tokens.push(token);
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() && text.chars().last().unwrap().is_alphanumeric() {
            let text = text.split_at(prev_text.len());
            self.prev_index = self.current_index;
            self.current_index = self.tokens.len();
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn decode_rest(&self) -> Result<Option<String>> {
        let prev_text = if self.tokens.is_empty() {
            String::new()
        } else {
            let tokens = &self.tokens[self.prev_index..self.current_index];
            self.decode(tokens)?
        };
        let text = self.decode(&self.tokens[self.prev_index..])?;
        if text.len() > prev_text.len() {
            let text = text.split_at(prev_text.len());
            Ok(Some(text.1.to_string()))
        } else {
            Ok(None)
        }
    }

    pub fn decode_all(&self) -> Result<String> {
        self.decode(&self.tokens)
    }

    pub fn get_token(&self, token_s: &str) -> Option<u32> {
        self.tokenizer.get_vocab(true).get(token_s).copied()
    }

    pub fn tokenizer(&self) -> &tokenizers::Tokenizer {
        &self.tokenizer
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
        self.prev_index = 0;
        self.current_index = 0;
    }
}

static SEED_RNG: u64 = 299792458;
pub fn hub_load_safetensors(
    repo: &hf_hub::api::sync::ApiRepo,
    json_file: &str,
) -> Result<Vec<std::path::PathBuf>> {
    let json_file = repo
        .get(json_file)
        .map_err(|e| anyhow::anyhow!("Error while getting JSON file: {e}"))?;
    let json_file = std::fs::File::open(json_file)?;
    let json: serde_json::Value = serde_json::from_reader(&json_file)
        .map_err(|e| anyhow::anyhow!("Error while getting JSON file: {e}"))?;
    let weight_map = match json.get("weight_map") {
        None => anyhow::bail!("no weight map in {json_file:?}"),
        Some(serde_json::Value::Object(map)) => map,
        Some(_) => anyhow::bail!("weight map in {json_file:?} is not a map"),
    };
    let mut safetensors_files = std::collections::HashSet::new();
    for value in weight_map.values() {
        if let Some(file) = value.as_str() {
            safetensors_files.insert(file.to_string());
        }
    }
    let safetensors_files = safetensors_files
        .iter()
        .map(|v| {
            repo.get(v)
                .map_err(|x| anyhow::anyhow!("Error while getting JSON file: {x}"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(safetensors_files)
}
