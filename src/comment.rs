extern crate mysql;

use mysql::prelude::*;
use mysql::*;
use serde::Serialize;

#[derive(Serialize, Debug)]
struct Comment {
    id: u32,
    author: String,
    email: Option<String>,
    website: Option<String>,
    remote_addr: String,
    created: String,
    parent: Option<u32>,
    text: String,
}

#[derive(Serialize, Debug)]
struct Thread {
    id: String,
    title: String,
    comments: Vec<Comment>,
}

fn main() -> Result<()> {
    let url = Opts::from_url("mysql://vjaquez@127.0.0.1/geeklog")?;
    let mut conn = Conn::new(url)?;

    let mut threads = conn.query_map(
        "select stories.sid as id,
                stories.title as title,
                count(*) as count
         from  stories, comments
         where comments.sid = stories.sid
         group by stories.sid
         order by comments.date",
        | (sid, title, count) : (String, String, usize) | Thread {
            id: format!("/{}", sid),
            title: title.replace(&['\u{91}', '\u{92}'], "'"),
            comments: Vec::with_capacity(count),
        },
    )?;

    for thread in threads.iter_mut() {
        thread.comments = conn.exec_map(
            "select comments.cid as id,
                    if (length(users.fullname)>0, users.fullname, users.username) as author,
                    users.email as mail,
                    users.homepage as homepage,
                    comments.ipaddress as remote_addr,
                    date_format(convert_tz(comments.date, '-06:00', '+00:00'), '%Y-%m-%d %T' )as created,
                    comments.pid as parent,
                    comments.comment as comment
                    from users, comments
                    where comments.uid = users.uid
                          and comments.sid = ?
                    order by created",
            (&thread.id[1..thread.id.len()],),
            | (id, author, mail, homepage, remote_addr, created, pid, comment) : (u32, String, String, Option<String>, String, String, u32, Vec<u8>) | {
                Comment {
                    id,
                    author,
                    email: if mail.len() == 0 { None } else { Some(mail) },
                    website: homepage.and_then(|website| if website.len() == 0 { None } else { Some(website) }),
                    remote_addr: if remote_addr.len() == 0 { "0.0.0.0".to_string() } else { remote_addr },
                    created,
                    parent: if pid == 0 { None } else { Some(pid) },
                    text: String::from_utf8_lossy(&comment).into_owned().replace(char::is_control, "").replace(char::is_whitespace, " "),
                }
            }
        ).unwrap();
    }

    let threads_json = serde_json::to_string(&threads).unwrap();
    println!("{}", threads_json);

    Ok(())
}
