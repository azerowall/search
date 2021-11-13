use std::sync::RwLock;
use std::sync::Arc;
use std::path::Path;

use actix_web::web::block;
use serde::Deserialize;

use crate::config;
use crate::index_config::IndexConfig;

pub struct AddDocRequest {
    pub doc: String,
    pub commit: bool,
}

pub struct DeleteByTermRequest {
    pub field: String,
    pub term: String,
    pub commit: bool,
}

#[derive(Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub limit: usize,
    pub offset: usize,
}

pub struct LocalIndex {
    schema: tantivy::schema::Schema,
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    writer: RwLock<tantivy::IndexWriter>,
}

pub fn create_index_in_dir(path: &Path, index_conf: &IndexConfig) -> crate::Result<tantivy::Index> {
    let index = tantivy::Index::builder()
        .settings(index_conf.settings.clone())
        .schema(index_conf.schema.clone())
        .create_in_dir(&path)?;

    let tokenizers = index.tokenizers();
    for analyzer in &index_conf.analyzers {
        tokenizers.register(&analyzer.name, analyzer.make_analyzer())
    }

    Ok(index)
}

impl LocalIndex {

    pub fn from_index(_name: String, index: tantivy::Index, config: &config::Search) -> crate::Result<LocalIndex> {
        let schema = index.schema();
        let reader = index.reader()?;
        let writer = if let Some(num_threads) = config.indexer_num_threads {
            index.writer_with_num_threads(num_threads, config.indexer_heap_size)
        } else {
            index.writer(config.indexer_heap_size)
        }?;
        Ok(LocalIndex {
            schema,
            index,
            reader,
            writer: RwLock::new(writer),
        })
    }

    pub async fn add_document(self: &Arc<Self>, req: AddDocRequest) -> crate::Result<()> {
        let doc = self.schema.parse_document(&req.doc)?;
        // TODO: если очередь заполнена, то вызов add_document может быть блокирующим
        self.writer
            .read()
            .map_err(crate::error::lock_poisoned)?
            .add_document(doc);
        if req.commit {
            let this = self.clone();
            block(move || -> crate::Result<()> {
                log::debug!("Committing add");
                this.writer
                    .write()
                    .map_err(crate::error::lock_poisoned)?
                    .commit()?;
                Ok(())
            })
            .await?;
        }
        Ok(())
    }

    pub async fn delete_by_term(self: &Arc<Self>, req: DeleteByTermRequest) -> crate::Result<()> {
        let DeleteByTermRequest {
            field: field_name,
            term,
            commit,
        } = req;

        let field = self
            .schema
            .get_field(&field_name)
            .ok_or(crate::error::field_not_exist(field_name))?;
        let field_entry = self.schema.get_field_entry(field);
        let field_type = field_entry.field_type();
        let term = crate::query::make_term(field, field_type, &term)?;

        self.writer
            .read()
            .map_err(crate::error::lock_poisoned)?
            .delete_term(term);

        if commit {
            let this = self.clone();
            block(move || -> crate::Result<_> {
                log::debug!("Committing delete");
                this.writer
                    .write()
                    .map_err(crate::error::lock_poisoned)?
                    .commit()?;
                Ok(())
            })
            .await?;
        }

        Ok(())
    }

    pub async fn update_document(&self) -> crate::Result<()> {
        todo!()
    }

    pub async fn search(
        self: &Arc<Self>,
        req: SearchRequest,
    ) -> crate::Result<Vec<tantivy::schema::NamedFieldDocument>> {
        let this = self.clone();
        block(move || -> crate::Result<_> {
            let searcher = this.reader.searcher();
            let query_parser = tantivy::query::QueryParser::for_index(&this.index, vec![]);
            let query = query_parser.parse_query(&req.query)?;
            let collector =
                tantivy::collector::TopDocs::with_limit(req.limit).and_offset(req.offset);
            let docs = searcher.search(&query, &collector)?;

            docs.iter()
                .map(|(_score, doc_address)| {
                    searcher
                        .doc(*doc_address)
                        .map(|doc| this.schema.to_named_doc(&doc))
                        .map_err(From::from)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .await
        .map_err(From::from)
    }
}
