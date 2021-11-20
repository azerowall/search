use tantivy::schema::{Schema as TantivySchema};
use tantivy::tokenizer::{
    TextAnalyzer, FacetTokenizer, NgramTokenizer, RawTokenizer, SimpleTokenizer,
    BoxTokenFilter, AlphaNumOnlyFilter, AsciiFoldingFilter, RemoveLongFilter, Stemmer, LowerCaser,
};
use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct NgramTokenizerConfig {
    /// min size of the n-gram
    min_gram: usize,
    /// max size of the n-gram
    max_gram: usize,
    /// if true, will only parse the leading edge of the input
    prefix_only: bool,
}

impl From<&NgramTokenizerConfig> for NgramTokenizer {
    fn from(config: &NgramTokenizerConfig) -> Self {
        Self::new(config.min_gram, config.max_gram, config.prefix_only)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum TokenizerConfig {
    Raw,
    Simple,
    Ngram(NgramTokenizerConfig),
    Facet,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveLongFilterConfig {
    limit: usize,
}

impl From<&RemoveLongFilterConfig> for RemoveLongFilter {
    fn from(conf: &RemoveLongFilterConfig) -> Self {
        Self::limit(conf.limit)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StemmerConfig {
    lang: tantivy::tokenizer::Language,
}

impl From<&StemmerConfig> for Stemmer {
    fn from(stemmer: &StemmerConfig) -> Self {
        Self::new(stemmer.lang)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum TokenFilterConfig {
    Lowercase,
    //Stop,
    RemoveLong(RemoveLongFilterConfig),
    AlphaNum,
    AsciiFolding,
    Stemmer(StemmerConfig),
}

impl TokenFilterConfig {
    pub fn make_token_filter(&self) -> BoxTokenFilter {
        match self {
            TokenFilterConfig::Lowercase => LowerCaser.into(),
            TokenFilterConfig::RemoveLong(conf) => RemoveLongFilter::from(conf).into(),
            TokenFilterConfig::AlphaNum => AlphaNumOnlyFilter.into(),
            TokenFilterConfig::AsciiFolding => AsciiFoldingFilter.into(),
            TokenFilterConfig::Stemmer(conf) => Stemmer::from(conf).into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    pub name: String,
    pub tokenizer: TokenizerConfig,
    #[serde(default)]
    pub token_filters: Vec<TokenFilterConfig>,
}

impl AnalyzerConfig {
    pub fn make_analyzer(&self) -> TextAnalyzer {
        let filters = self.token_filters
            .iter()
            .map(TokenFilterConfig::make_token_filter)
            .collect::<Vec<_>>();

        use TokenizerConfig::*;
        match &self.tokenizer {
            Raw => TextAnalyzer::new(RawTokenizer, filters),
            Simple => TextAnalyzer::new(SimpleTokenizer, filters),
            Ngram(conf) => TextAnalyzer::new(NgramTokenizer::from(conf), filters),
            Facet => TextAnalyzer::new(FacetTokenizer, filters),
        }
    }
}

pub type Analyzers = Vec<AnalyzerConfig>;

#[derive(Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default)]
    pub settings: tantivy::IndexSettings,
    #[serde(default)]
    pub analyzers: Analyzers,
    pub schema: TantivySchema,
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_index_config_deserialize() {
        let config = r#"
{
    "schema": []
}
        "#;
        let config: IndexConfig = serde_json::from_str(config).unwrap();

        assert_eq!(config.analyzers.len(), 0);
        assert_eq!(config.schema.fields().count(), 0);
    }

    #[test]
    fn test_index_config_with_custom_analyzer_deserialize() {
        let config = r#"
{
    "analyzers": [
        {
            "name": "en_stem",
            "tokenizer": {
                "type": "ngram",
                "min_gram": 1,
                "max_gram": 3,
                "prefix_only": true
            },
            "token_filters": [{
                "type": "stemmer",
                "lang": "English"
            }]
        }
    ],
    "schema": []
}
        "#;
        let config: IndexConfig = serde_json::from_str(config).unwrap();

        let analyzer = &config.analyzers[0];
        assert!(matches!(analyzer.tokenizer, TokenizerConfig::Ngram(_)));
        let filter = &analyzer.token_filters[0];
        match filter {
            TokenFilterConfig::Stemmer(conf) => {
                assert_eq!(conf.lang, tantivy::tokenizer::Language::English);
            }
            _ => panic!("Expected token filter 'stemmer'")
        }
    }
}