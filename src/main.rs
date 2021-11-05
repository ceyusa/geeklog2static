extern crate mysql;
extern crate pandoc;

use mysql::prelude::*;
use mysql::*;
use pandoc::*;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

#[derive(Debug, PartialEq)]
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
    fn setup(&self) -> std::io::Result<()> {
        create_dir_all("content")
    }
    fn write(&self, text: String) -> std::io::Result<()> {
        let fsname = &format!("content/{}.md", self.slug);
        let path = Path::new(&fsname);
        let mut file = File::create(&path)?;
        file.write_all(text.as_bytes())
    }
    fn compose(&self, text: String) -> String {
        String::from(format!(
            "\
+++
title = {}
slug = {}
author = {}
date = {}
[taxonomies]
topic = {}
+++

{}
",
            self.title, self.slug, self.author, self.date, self.topic, text
        ))
    }
    fn convert(&self) -> String {
        let mut pandoc = pandoc::new();
        pandoc
            .set_input(InputKind::Pipe(self.text.clone()))
            .set_output_format(
                OutputFormat::MarkdownGithub,
                vec![
                    MarkdownExtension::FencedCodeBlocks,
                    MarkdownExtension::LineBlocks,
                    MarkdownExtension::GridTables,
                    MarkdownExtension::FancyLists,
                    MarkdownExtension::DefinitionLists,
                ],
            )
            .set_output(OutputKind::Pipe);

        if self.mode == PostMode::HTML {
            pandoc.set_input_format(InputFormat::Html, Vec::new());
        }

        match pandoc.execute() {
            Ok(PandocOutput::ToBuffer(text)) => text,
            Ok(_) => {
                eprintln!("Wrong output for article {}", self.title);
                String::new()
            }
            Err(e) => {
                eprintln!("Conversion failed for article {}: {}", self.title, e);
                String::new()
            }
        }
    }
    fn process(&self) -> std::io::Result<()> {
        self.setup()?;
        self.write(self.compose(self.convert()))
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
        Ok(row) => Article::new(row).process().unwrap_or_else(|err| {
            eprintln!("Failed to process article: {}", err);
        }),
        Err(err) => eprintln!("Error: {}", err),
    });

    Ok(())
}
