use std::convert::TryFrom;
use std::{collections::HashMap, sync::Arc};
use std::sync::{RwLock};
use std::path::PathBuf;

use actix_web::web::block;
use serde::{Deserialize};

use anyhow::{Context, anyhow};

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

#[derive(Deserialize)]
pub struct IndexConfig {
    pub settings: tantivy::IndexSettings,
    pub schema: tantivy::schema::Schema,
}

pub struct LocalIndex {
    name: String,
    schema: tantivy::schema::Schema,
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    writer: RwLock<tantivy::IndexWriter>,
}

impl LocalIndex {

    pub fn from_index(name: String, index: tantivy::Index) -> crate::Result<LocalIndex> {
        let schema = index.schema();
        let reader = index.reader()?;
        let writer = index.writer(50_000_000)?;
        Ok(LocalIndex {
            name,
            schema,
            index,
            reader,
            writer: RwLock::new(writer),
        })
    }

    pub async fn add_document(self: &Arc<Self>, req: AddDocRequest) -> crate::Result<()> {
        let doc = self.schema.parse_document(&req.doc)?;
        // TODO: если очередь заполнена, то вызов add_document может быть блокирующим
        self.writer.read()
            .map_err(|_| anyhow!("poison error"))?
            .add_document(doc);
        if req.commit {
            let this = self.clone();
            block(move || -> crate::Result<()> {
                log::debug!("Committing add");
                this.writer.write()
                    .map_err(|_| anyhow!("poison error"))?
                    .commit()?;
                Ok(())
            }).await?;
        }
        Ok(())
    }

    pub async fn delete_by_term(self: &Arc<Self>, req: DeleteByTermRequest) -> crate::Result<()> {
        let DeleteByTermRequest {
            field: field_name,
            term,
            commit,
        } = req;

        let field = self.schema.get_field(&field_name)
            .context(format!("Field '{}' not exist", &field_name))?;
        let field_entry = self.schema.get_field_entry(field);
        let field_type = field_entry.field_type();
        let term = crate::query::make_term(field, field_type, &term)?;

        self.writer.read()
            .map_err(|_| anyhow!("poison error"))?
            .delete_term(term);

        if commit {
            let this = self.clone();
            block( move || -> crate::Result<_> {
                log::debug!("Committing delete");
                this.writer.write()
                    .map_err(|_| anyhow!("poison error"))?
                    .commit()?;
                Ok(())
            }).await?;
        }

        Ok(())
    }

    pub async fn update_document(&self) -> crate::Result<()> {
        todo!()
    }

    pub async fn search(self: &Arc<Self>, req: SearchRequest) -> crate::Result<Vec<tantivy::schema::NamedFieldDocument>> {
        let this = self.clone();
        block( move || -> crate::Result<_> {
            let searcher = this.reader.searcher();
            let query_parser = tantivy::query::QueryParser::for_index(&this.index, vec![]);
            let query = query_parser.parse_query(&req.query)?;
            let collector = tantivy::collector::TopDocs::with_limit(req.limit)
                .and_offset(req.offset);
            let docs = searcher.search(&query, &collector)?;
    
            docs.iter()
                .map(|(_score, doc_address)| {
                    searcher.doc(*doc_address)
                        .map(|doc| this.schema.to_named_doc(&doc))
                        .map_err(From::from)
                })
                .collect::<crate::Result<Vec<_>,_>>()
        })
        .await
        .map_err(From::from)
    }
}


#[cfg(test)]
mod test {
    use std::time::{Duration};
    use std::thread::sleep;

    use super::*;
    use tantivy::Index;
    use actix_web::rt::time::delay_for;
    use crate::test;


    #[actix_rt::test]
    async fn test_add_delete_search() -> crate::Result<()> {
        std::env::set_var("RUST_LOG", "debug");
        pretty_env_logger::init();
        
        let index = Index::builder()
            .schema(test::make_test_schema())
            .create_in_ram()?;
        let index = Arc::new(LocalIndex::from_index("test".into(), index)?);

        for i in 0..5 {
            index.add_document(AddDocRequest {
                doc: format!(r#"{{ "id": {}, "text": "test text" }}"#, i),
                commit: i == 4,
            }).await?;
        }

        //sleep(Duration::from_secs(3));

        let docs = index.search(SearchRequest {
            query: "text:test".into(),
            offset: 0,
            limit: 5,
        }).await?;
        log::debug!("{} docs", docs.len());
        assert_eq!(docs.len(), 5);

        index.delete_by_term(DeleteByTermRequest {
            field: "id".into(),
            term: "3".into(),
            commit: true,
        }).await?;

        let docs = index.search(SearchRequest {
            query: "text:test".into(),
            offset: 0,
            limit: 5,
        }).await?;
        log::debug!("{} docs", docs.len());
        assert_eq!(docs.len(), 4);
        
        Ok(())
    }
}