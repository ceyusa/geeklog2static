extern crate mysql;

use mysql::prelude::*;
use mysql::*;

#[derive(Debug)]
enum PostMode {
    Unknown,
    Text,
    HTML,
}

#[derive(Debug)]
struct Article {
    slug: String,
    topic: String,
    title: String,
    author: String,
    date: String,
    mode: PostMode,
    text: String,
}

impl FromRow for Article {
    fn from_row_opt(row: Row) -> std::result::Result<Article, FromRowError> {
        Ok(Article {
            slug: row
                .get_opt(0)
                .unwrap()
                .map_err(|_| FromRowError(row.clone()))?,
            topic: row
                .get_opt(1)
                .unwrap()
                .map_err(|_| FromRowError(row.clone()))?,
            title: row
                .get_opt(2)
                .unwrap()
                .map_err(|_| FromRowError(row.clone()))?,
            author: row
                .get_opt::<String, usize>(3)
                .unwrap()
                .map(|v| match v.as_str() {
                    "" => row
                        .get_opt(4)
                        .unwrap()
                        .or::<String>(Ok(String::from("Anónimo")))
                        .unwrap(),
                    _ => v,
                })
                .or_else(|_| {
                    row.get_opt(4)
                        .unwrap()
                        .or::<String>(Ok(String::from("Anónimo")))
                })
                .unwrap(),
            date: row
                .get_opt(5)
                .unwrap()
                .map_err(|_| FromRowError(row.clone()))?,
            mode: row
                .get_opt::<String, usize>(6)
                .unwrap()
                .map(|v| match v.as_str() {
                    "plaintext" => Ok(PostMode::Text),
                    "html" => Ok(PostMode::HTML),
                    _ => Ok(PostMode::Unknown),
                })
                .map_err(|_| FromRowError(row.clone()))??,
            text: format!(
                "{}{}",
                row.get_opt::<String, usize>(7)
                    .unwrap()
                    .map_err(|_| FromRowError(row.clone()))?,
                row.get_opt(8)
                    .unwrap()
                    .or::<String>(Ok(String::from("")))
                    .unwrap(),
            ),
        })
    }
}

impl Article {
    fn new(row: Row) -> Self {
        from_row::<Article>(row)
    }
    fn dump(&self) {
        println!("{:?}", self);
    }
}

fn main() -> Result<()> {
    let url = Opts::from_url("mysql://vjaquez@localhost/geeklog")?;
    let mut conn = Conn::new(url)?;

    let query = conn.query_iter(
        "\
	SELECT s.sid, t.tid, s.title, u.fullname, u.username, s.date,
               s.postmode, s.introtext, s.bodytext
	FROM stories AS s INNER JOIN users AS u ON s.uid = u.uid
                          INNER JOIN topic_assignments as t ON t.id = s.sid
        ORDER BY s.date
",
    )?;

    query.for_each(|row| match row {
        Ok(row) => Article::new(row).dump(),
        Err(err) => println!("Error: {}", err),
    });

    Ok(())
}
