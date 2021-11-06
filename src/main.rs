//#![allow(dead_code, unused_imports)]

mod api;
mod auth;
mod access_control;
mod error;
mod index;
mod index_manager;
mod query;

#[cfg(test)]
mod test;

pub use crate::error::Error;
pub type Result<T, E = crate::error::Error> = std::result::Result<T, E>;

/*
    TODO:
    IndexManager - RwLock внутри или снаружи?
    IndexWriter под Arc или весь LocalIndex под Arc?
    commit каждую секунду
    тестирование - setup/teardown
    конфиг
    dto
    ленивая инициализация IndexReader и IndexWriter
    разделение Scheme и LocalIndex - Scheme может храниться даже если самого индекса на этой ноде нет.
    primary key ?
    кластер: шардинг, репликация
    шифрование трафика api
    шифрование трафика кластера
    аутентификация/авторизация
    пользователи и права - а нужно ли?
*/

#[actix_web::main]
async fn main() -> crate::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    pretty_env_logger::init();

    api::run_server().await?;
    Ok(())
}

#[cfg(test)]
mod test1 {

    use tantivy::doc;
    use tantivy::schema::{Schema, INDEXED, STORED, TEXT};

    #[test]
    fn test_tantivy() {
        let mut schema = Schema::builder();
        let id = schema.add_u64_field("id", INDEXED | STORED);
        let text = schema.add_text_field("text", TEXT);
        let schema = schema.build();

        let index = tantivy::Index::builder()
            .schema(schema)
            .create_in_dir("/tmp/test_index")
            .unwrap();

        let mut writer = index.writer(3000000).unwrap();
        let reader = index.reader().unwrap();

        for i in 0u64..10 {
            writer.add_document(doc!(
                id => i,
                text => format!("test test"),
            ));
        }
        writer.commit().unwrap();

        writer.delete_term(tantivy::Term::from_field_u64(id, 3));
        writer.commit().unwrap();

        //writer.wait_merging_threads().unwrap();

        reader.reload().unwrap();

        let searcher = reader.searcher();
        let collector = tantivy::collector::TopDocs::with_limit(100);
        let query_parser = tantivy::query::QueryParser::for_index(&index, vec![text]);
        let query = query_parser.parse_query("test").unwrap();
        let docs = searcher.search(&query, &collector).unwrap();
        assert_eq!(docs.len(), 9);

        //writer.commit().unwrap();
    }
}
