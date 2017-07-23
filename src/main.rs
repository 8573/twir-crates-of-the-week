extern crate itertools;
extern crate time;
extern crate serde;
extern crate serde_yaml;
extern crate slog_async;
extern crate slog_term;

#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate slog;

use itertools::Itertools;
use std::fmt;
use std::fs;
use std::fs::File;
use std::io;
use std::io::BufWriter;
use std::io::Write;
use time::Duration;
use time::Tm;

lazy_static! {
    static ref LOG: slog::Logger = {
        use slog::Drain;
        let decorator = slog_term::PlainSyncDecorator::new(io::stderr());
        let drain = slog_term::CompactFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, o!())
    };
}

fn main() {
    match (|| -> Result<_> {
                 process_cotw_list()?;
                 Ok(())
             })() {
        Ok(()) => {}
        Err(err) => panic!("Exiting on error: {:#?}", err),
    }
}

error_chain! {
    foreign_links {
        Io(io::Error);
        TimeParse(time::ParseError);
        SerdeYaml(serde_yaml::Error);
    }

    errors {
        CotwEntryMissingDate(crate_id: String) {
            description("a Crate of the Week entry is missing the `date` field")
            display("The Crate of the Week entry for crate {:?} is missing the `date` field.",
                    crate_id)
        }
        CotwEntryMissingId(entry_date: String) {
            description("a Crate of the Week entry is missing the `id` field")
            display("The Crate of the Week entry dated {:?} is missing the `id` field.",
                    entry_date)
        }
    }
}

#[derive(Debug)]
struct CotwEntry {
    date: Tm,
    id: Option<String>,
    url: Option<String>,
}

fn process_cotw_list() -> Result<()> {
    let list = read_cotw_list()?;

    validate_cotw_list(&list)?;

    write_cotw_list(list)?;

    Ok(())
}

fn read_cotw_list() -> Result<Vec<CotwEntry>> {
    serde_yaml::from_reader(File::open("TWiR-CotW-list.yaml")?).map_err(Into::into)
}

fn validate_cotw_list(list: &[CotwEntry]) -> Result<()> {
    for (prev,
         &CotwEntry {
             ref date,
             ref id,
             url: _,
         }) in list.iter().tuple_windows()
    {
        if date == &prev.date {
            warn!(LOG,
                   "Crate of the Week entry is dated the same as preceding entry; this may be a \
                    typo";
                   "date" => date.strftime("%F")?.to_string(), "crate" => id);
        }

        if !(date >= &prev.date) {
            error!(LOG,
                   "Crate of the Week entry is out of order (it follows an entry that has a later \
                    date)";
                   "date" => date.strftime("%F")?.to_string(), "crate" => id);
        }

        if date >= &(prev.date + Duration::weeks(2)) {
            warn!(LOG,
                  "Crate of the Week entry is dated two or more weeks later than preceding entry; \
                   one or more entries may be missing";
                  "date" => date.strftime("%F")?.to_string(), "crate" => id);
        }
    }

    Ok(())
}

fn write_cotw_list(list: Vec<CotwEntry>) -> Result<()> {
    fs::create_dir_all("built")?;

    let mut file = BufWriter::new(File::create("built/TWiR-CotW-list.adoc")?);

    writeln!(
        file,
        r###"
= _This Week in Rust_`'s Crates of the Week

The Rust crates that have been honored by link:https://this-week-in-rust.org[_This Week in Rust_]
as "`Crate of the Week`".

[%autowidth]
|===
| Date | Crate
"###
    )?;

    for CotwEntry { date, id, url } in list {
        if let Some(crate_id) = id {
            writeln!(
                file,
                "| {date} | link:{url}[{id}]\n",
                date = date.strftime("%F")?,
                id = crate_id,
                url = url.unwrap_or_else(|| format!("https://crates.io/crates/{id}", id = crate_id)),
            )?;
        }
    }

    writeln!(file, "|===")?;

    Ok(())
}

impl<'de> serde::Deserialize<'de> for CotwEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de;

        #[derive(Debug, Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            Date,
            Id,
            Nominator,
            Note,
            Url,
        }
        const FIELDS: &'static [&'static str] = &["date", "id", "nominator", "note", "url"];

        struct CotwEntryVisitor;

        impl<'vde> de::Visitor<'vde> for CotwEntryVisitor {
            type Value = CotwEntry;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    formatter,
                    "a record of the form `{{date: YYYY-MM-DD, id: crate-name}}`"
                )
            }

            fn visit_map<V>(self, mut map_access: V) -> std::result::Result<Self::Value, V::Error>
            where
                V: de::MapAccess<'vde>,
            {
                use serde::de::Error as E;

                let mut date: Option<String> = None;
                let mut id = None;
                let mut url = None;

                while let Some(key) = map_access.next_key()? {
                    match key {
                        Field::Date => {
                            match date {
                                None => date = Some(map_access.next_value()?),
                                Some(_) => return Err(E::duplicate_field("date")),
                            }
                        }
                        Field::Id => {
                            match id {
                                None => id = Some(map_access.next_value()?),
                                Some(_) => return Err(E::duplicate_field("id")),
                            }
                        }
                        Field::Nominator => {
                            // TODO
                            let _: String = map_access.next_value()?;
                        }
                        Field::Note => {
                            // TODO
                            let _: String = map_access.next_value()?;
                        }
                        Field::Url => {
                            match url {
                                None => url = Some(map_access.next_value()?),
                                Some(_) => return Err(E::duplicate_field("url")),
                            }
                        }
                    }
                }

                Ok(CotwEntry {
                    date: date.ok_or(E::missing_field("date")).and_then(|ref s| {
                        time::strptime(s, "%F").or(Err(E::invalid_value(
                            de::Unexpected::Str(s),
                            &"a date of the form YYYY-MM-DD",
                        )))
                    })?,
                    id: id.ok_or(E::missing_field("id"))?,
                    url,
                })
            }
        }

        deserializer.deserialize_struct("CotwEntry", FIELDS, CotwEntryVisitor)
    }
}
