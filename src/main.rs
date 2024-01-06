use anyhow::Result;
use chrono_tz::Europe::Paris;
use console::style;
use serde::Deserialize;
use std::fmt;
use std::process::Command;
use structopt::StructOpt;

#[derive(Deserialize, Debug)]
struct TrainGps {
    speed: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Stop {
    label: String,
    theoric_date: chrono::DateTime<chrono::Utc>,
    real_date: chrono::DateTime<chrono::Utc>,
    is_delayed: bool,
    is_created: bool,
    is_diversion: bool,
    is_removed: bool,
}

impl Stop {
    fn in_the_past(&self) -> bool {
        let now = chrono::Utc::now();
        now > self.real_date + chrono::Duration::minutes(5)
    }
    fn theoric(&self) -> String {
        let local_time = self.theoric_date.with_timezone(&Paris);
        let local_time = local_time.format("%H:%M");
        if self.is_delayed {
            style(local_time).red()
        } else {
            style(local_time).green()
        }
        .to_string()
    }

    fn pango_theoric(&self) -> String {
        let local_time = self.theoric_date.with_timezone(&Paris);
        let local_time = local_time.format("%H:%M");
        if self.is_delayed {
            format!("<span foreground=\\\"red\\\"><s>{local_time}</s></span>")
        } else {
            format!("<span foreground=\\\"green\\\">{local_time}</span>")
        }
    }

    fn real(&self) -> String {
        let local_time = self.real_date.with_timezone(&Paris);
        if self.is_delayed {
            style(local_time.format("%H:%M")).green().to_string()
        } else {
            String::new()
        }
    }

    fn pango_real(&self) -> String {
        let local_time = self.real_date.with_timezone(&Paris);
        if self.is_delayed {
            format!("<span foreground=\\\"green\\\">{local_time}</span>")
        } else {
            String::new()
        }
    }

    fn formated_label(&self) -> String {
        let status = if self.in_the_past() {
            style(" ").white()
        } else if self.is_created {
            style("+").green()
        } else if self.is_removed {
            style("-").red()
        } else if self.is_diversion {
            style("~").yellow()
        } else {
            style("·")
        };

        format!("{} {}", status.bold(), self.label)
    }

    fn pango_formated_label(&self) -> String {
        let (status, color) = if self.in_the_past() {
            (" ", "grey")
        } else if self.is_created {
            ("+", "green")
        } else if self.is_removed {
            ("-", "red")
        } else if self.is_diversion {
            ("~", "yellow")
        } else {
            ("·", "white")
        };

        format!(
            "<span foreground=\\\"{color}\\\"><b>{status}</b></span> {}",
            self.label.replace("&", "&amp;")
        )
    }
}

impl fmt::Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.theoric(),
            self.real(),
            self.formated_label(),
        )
    }
}

#[derive(Deserialize, Debug)]
struct Trip {
    stops: Vec<Stop>,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "inouifi", about = "Get information about the current train.")]
enum Inouifi {
    Connected {
        #[structopt(short, long)]
        quiet: bool,
    },
    Speed {
        #[structopt(short, long)]
        no_units: bool,
    },
    Stops,
    Waybar,
}

fn connected() -> Result<bool> {
    let result = Command::new("iwconfig").output()?;
    let result = String::from_utf8(result.stdout)?;
    Ok(result.contains("_SNCF_WIFI_INOUI"))
}

fn display_connected(quiet: bool) -> i32 {
    match connected() {
        Ok(true) => {
            if !quiet {
                println!("Connected to the train wifi");
            }
            0
        }
        _ => {
            if !quiet {
                println!("Not connected to train wifi");
            }
            1
        }
    }
}

fn pango_format() -> i32 {
    let speed = speed();
    let trip = trip();
    match (speed, trip) {
        (Ok(speed), Ok(trip)) => {
            let stops: Vec<_> = trip
                .stops
                .iter()
                .map(|s| {
                    format!(
                        "{}{} {}",
                        s.pango_theoric(),
                        s.pango_real(),
                        s.pango_formated_label()
                    )
                })
                .collect();

            let str = format!(
                r#"{{"text": "{speed}", "tooltip": "{}"}}"#,
                stops.join("\\r")
            );
            println!("{str}");
            0
        }
        _ => 5,
    }
}

fn speed() -> Result<i32> {
    let resp =
        reqwest::blocking::get("https://wifi.sncf/router/api/train/gps")?.json::<TrainGps>()?;

    // The api is in the metric system (YAY)
    // Thats m/s, let’s convert it to km/h
    Ok((resp.speed * 3.6) as i32)
}

fn display_speed(no_units: bool) -> i32 {
    match speed() {
        Ok(value) => {
            if no_units {
                print!("{value}");
            } else {
                println!("{value} km/h");
            }
            0
        }
        _ => {
            eprintln!("Could fetch the speed, are you connected to the train’s wifi?");
            3
        }
    }
}

fn trip() -> Result<Trip> {
    let r = reqwest::blocking::get("https://wifi.sncf/router/api/train/details")?
        .json::<Trip>()
        .unwrap();
    Ok(r)
}

fn display_trip() -> i32 {
    match trip() {
        Ok(trip) => {
            for stop in trip.stops {
                println!("{stop}");
            }
            0
        }
        _ => {
            eprintln!("Could not fetch train details, are you connected to the train’s wifi?");
            4
        }
    }
}

fn main() {
    let exit_code = match Inouifi::from_args() {
        Inouifi::Connected { quiet } => display_connected(quiet),
        Inouifi::Speed { no_units } => display_speed(no_units),
        Inouifi::Stops => display_trip(),
        Inouifi::Waybar => pango_format(),
    };

    std::process::exit(exit_code);
}
