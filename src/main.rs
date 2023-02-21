extern crate mysql;
extern crate pandoc;

use mysql::prelude::*;
use mysql::*;
use pandoc::*;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::thread;

struct Content {
    slug: String,
    text: String,
}

enum Message {
    Write(Content),
    Quit,
}

struct Writer {
    tx: Sender<Message>,
}

impl Writer {
    fn new(dir: String) -> Result<Self> {
        let _dir = dir.clone();
        create_dir_all(dir)?;
        let (tx, rx) = channel();
        let _ = thread::spawn(move || loop {
            match rx.recv().unwrap() {
                Message::Quit => break,
                Message::Write(content) => {
                    let fsname = &format!("{}/{}.md", _dir, content.slug);
                    let path = Path::new(&fsname);
                    File::create(path)
                        .and_then(|mut file| file.write_all(content.text.as_bytes()))
                        .unwrap_or_else(|err| eprintln!("Failed to write article: {}", err));
                }
            }
        });
        Ok(Writer { tx })
    }
    fn write(&self, content: Content) {
        self.tx
            .send(Message::Write(content))
            .unwrap_or_else(|err| eprintln!("Failed write article: {}", err));
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.tx.send(Message::Quit).unwrap();
    }
}

#[derive(Debug, PartialEq)]
enum PostMode {
    Unknown,
    Text,
    Html,
}

#[derive(Debug)]
struct Article {
    slug: String,
    topic: String,
    title: String,
    fullname: Option<String>,
    username: Option<String>,
    date: String,
    mode: PostMode,
    summary: String,
    text: Option<String>,
}

impl FromRow for Article {
    fn from_row_opt(row: Row) -> std::result::Result<Article, FromRowError> {
        Ok(Article {
            slug: row
                .get(0)
                .ok_or_else(|| FromRowError(row.clone()))?,
            topic: row
                .get(1)
                .ok_or_else(|| FromRowError(row.clone()))?,
            title: row
                .get::<String, usize>(2)
                .map(|v| {
                    if v.contains('\\') {
                        v.replace("\\", "")
                    } else if v.contains('\r') {
                        v.replace("\r\n", " ")
                    } else {
                        v
                    }
                })
                .ok_or_else(|| FromRowError(row.clone()))?,
            fullname: row
                .get_opt::<String, usize>(3)
                .unwrap()
                .ok()
                .and_then(|v| match v.trim() {
                    "" => None,
                    _ => Some(v),
                }),
            username: row
                .get(4),
            date: row
                .get(5)
                .ok_or_else(|| FromRowError(row.clone()))?,
            mode: row
                .get::<String, usize>(6)
                .map(|v| match v.as_str() {
                    "plaintext" => PostMode::Text,
                    "html" => PostMode::Html,
                    _ => PostMode::Unknown,
                })
                .ok_or_else(|| FromRowError(row.clone()))?,
            summary: row
                .get(7)
                .ok_or_else(|| FromRowError(row.clone()))?,
            text: row
                .get::<String, usize>(8)
                .and_then(|v| match v.trim() {
                    "" => None,
                    _ => Some(v),
                }),
        })
    }
}

impl Article {
    fn new(row: Row) -> Self {
        from_row::<Article>(row)
    }
    fn compose(&self) -> Content {
        Content {
            slug: self.slug.clone(),
            text: format!(
                "\
+++
title = \"{}\"
slug = \"{}\"
date = \"{}\"
[taxonomies]
tema = [\"{}\"]
autor = [\"{}\"]
+++

{}
",
                self.title,
                self.slug,
                self.date,
                self.topic,
                self.author(),
                self.convert()
            ),
        }
    }
    fn author(&self) -> String {
        let anon = String::from("Anónimo");
        self.fullname.as_ref().or(self.username.as_ref()).unwrap_or(&anon).to_string()
    }
    fn convert(&self) -> String {
        self.text.as_ref()
            .map_or(self.run_pandoc(&self.summary),
                    |t| format!("{}\n<!-- more -->\n{}",
                                self.run_pandoc(&self.summary),
                                self.run_pandoc(&t))
            )
    }
    fn run_pandoc(&self, text: &String) -> String {
        let mut pandoc = pandoc::new();
        pandoc
            .set_input(InputKind::Pipe(text.to_string()))
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

        if self.mode == PostMode::Html {
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
}

fn main() -> Result<()> {
    let writer = Writer::new("content".to_string())?;

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
        Ok(row) => writer.write(Article::new(row).compose()),
        Err(err) => eprintln!("SQL Error: {}", err),
    });

    Ok(())
}
