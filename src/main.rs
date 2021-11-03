extern crate mysql;

use mysql::prelude::*;
use mysql::*;

#[derive(Debug)]
struct Article {
    slug: String,
    topic: String,
    title: String,
    author: String,
    date: String,
}

impl FromRow for Article {
    fn from_row_opt(row: Row) -> std::result::Result<Article, FromRowError> {
        let article = Article {
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
                .get_opt(3)
                .unwrap()
                .map(|v| {
                    if v == "" {
                        row.get_opt(4)
                            .unwrap()
                            .or::<String>(Ok(String::from("Anónimo")))
                            .unwrap()
                    } else {
                        v
                    }
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
        };
        Ok(article)
    }
}

fn process(article: &Article) {
    println!("{:?}", article);
}

fn main() -> Result<()> {
    let url = Opts::from_url("mysql://vjaquez@localhost/geeklog")?;
    let mut conn = Conn::new(url)?;

    let query = conn.query_iter(
        "\
	SELECT s.sid, t.tid, s.title, u.fullname, u.username, s.date
	FROM stories AS s INNER JOIN users AS u ON s.uid = u.uid
                          INNER JOIN topic_assignments as t ON t.id = s.sid
        ORDER BY s.date
",
    )?;

    query.for_each(|row| match row {
        Ok(row) => process(&from_row::<Article>(row)),
        Err(err) => println!("Error: {}", err),
    });

    Ok(())
}
