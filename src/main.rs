extern crate clap;
extern crate rouille;

use clap::{App, Arg};
use rouille::{router, Response};
use std::fs::File;
use std::io::prelude::*;
use std::net::ToSocketAddrs;
use std::sync::Mutex;

const INPUT_ARG: &str = "INPUT";
const COMPRESSIONFACTOR_ARG: &str = "COMPRESSIONFACTOR";
const LISTENADDR_ARG: &str = "LISTENADDR";

fn main() {
    let matches = App::new("Prometheus Painter")
        .version("0.1")
        .author("Giedrius Statkevičius <giedriuswork@gmail.com>")
        .about("Lets you paint ASCII in Prometheus")
        .arg(
            Arg::with_name(INPUT_ARG)
                .short("i")
                .long("input")
                .value_name(INPUT_ARG)
                .help("Sets the input file to use")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(COMPRESSIONFACTOR_ARG)
                .short("c")
                .long("compressionfactor")
                .value_name(COMPRESSIONFACTOR_ARG)
                .help("Integer factor of how much to compress the painting")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name(LISTENADDR_ARG)
                .short("l")
                .long("listen")
                .value_name(LISTENADDR_ARG)
                .help("IP:PORT pair on where to listen for HTTP connections")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let inputfile = matches.value_of(INPUT_ARG).unwrap();
    let listenaddr = matches
        .value_of(LISTENADDR_ARG)
        .unwrap_or("0.0.0.0:1234")
        .to_socket_addrs()
        .unwrap()
        .next()
        .unwrap();
    let compressionfactor = matches
        .value_of(COMPRESSIONFACTOR_ARG)
        .unwrap_or("2")
        .parse::<f64>()
        .unwrap_or(2.0);

    let mut contents = String::new();
    let mut f = File::open(inputfile).expect("file not found");
    f.read_to_string(&mut contents)
        .expect("failed to read specified file");

    let counter = Mutex::new(0);

    rouille::start_server(listenaddr, move |request| {
        let result = router!(request,
                             (GET) (/metrics) => {
                                let mut response_metrics = Vec::<String>::new();
                                let ctr = *counter.lock().unwrap();

                                println!("Got a request, character counter is {}", ctr);

                                let num_lns = contents.chars().filter(|&c| c == '\n').count();
                                let lns = contents.lines().enumerate();
                                let mut hit = false;
                                for (i, l) in lns {
                                    if ctr < l.len() {
                                        match l.chars().nth(ctr) {
                                            Some(' ') => {
                                                hit = true;
                                            },
                                            None => {},
                                            Some(_) => {
                                                let mtrc = format!("painting{{filename=\"{}\",line=\"{}\"}} {:.2}", matches.value_of(INPUT_ARG).unwrap(), i, ((num_lns as f64) - (i as f64)/compressionfactor));
                                                response_metrics.push(mtrc);
                                                hit = true;
                                            }
                                        }
                                    }
                                }

                                if !hit {
                                    println!("ended sending metrics");
                                    std::process::exit(0);
                                }

                                *counter.lock().unwrap() += 1;
                                response_metrics.join("\n").to_string()
                             },

                             _ => "Prometheus Painter".to_string()
        );

        Response::text(result)
    });
}
