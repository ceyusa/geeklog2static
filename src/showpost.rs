extern crate mysql;
use mysql::prelude::*;
use mysql::*;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Missing story id: {} <sid>", args.get(0).unwrap());
        std::process::exit(-1);
    }

    let url = Opts::from_url("mysql://vjaquez@localhost/geeklog")?;
    let mut conn = Conn::new(url)?;

    let sid = &args[1];

    let row: Option<Row> = conn.exec_first(
        "SELECT t.tid as topic,
                s.title as title,
                u.fullname as fullname,
                u.username as username,
                s.date as date,
                s.introtext as intro,
                s.bodytext as body
	      FROM stories AS s INNER JOIN users AS u ON s.uid = u.uid
                          INNER JOIN topic_assignments as t ON t.id = s.sid
        WHERE s.sid  = ?",
        (sid,),
    )?;

    match row {
        None => {
            eprintln!("No story with id {}", sid);
            std::process::exit(-1);
        }
        Some(row) => {
            let topic = row
                .get::<String, &str>("topic")
                .unwrap_or("*sin tópico*".to_string());
            let title = row
                .get::<String, &str>("title")
                .unwrap_or("*sin título*".to_string());
            let fullname = row
                .get::<String, &str>("fullname")
                .unwrap_or("*sin nombre completo*".to_string());
            let username = row
                .get::<String, &str>("username")
                .unwrap_or("*sin nombre de usuario*".to_string());
            let date: time::PrimitiveDateTime = row.get("date").unwrap();
            let intro = row.get::<String, &str>("intro").unwrap_or("".to_string());
            let body = row.get::<String, &str>("body").unwrap_or("".to_string());
            println!(
                "{}\n\n{}\n{}\n{}\t@{}\n\n{}\n\n----\n\n{}",
                title, topic, date, fullname, username, intro, body
            );
        }
    }

    Ok(())
}
