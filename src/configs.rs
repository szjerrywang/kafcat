use clap::crate_version;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::ArgMatches;
use log::LevelFilter;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Debug;
use std::str::FromStr;
use strum::Display;
use strum::EnumString;

const GROUP_ID_DEFAULT: &str = "kafcat";
const BROKERS_DEFAULT: &str = "localhost:9092";
const MSG_DELIMITER_DEFAULT: &str = "\n";
const KEY_DELIMITER_DEFAULT: &str = ":";
const OFFSET_DEFAULT: &str = "beginning";
const FORMAT_DEFAULT: &str = "text";

pub fn group_id() -> Arg<'static> {
    Arg::new("group-id")
        .short('G')
        .long("group-id")
        .about("Consumer group id. (Kafka >=0.9 balanced consumer groups)")
        .default_value(GROUP_ID_DEFAULT)
}

pub fn offset() -> Arg<'static> {
    Arg::new("offset")
        .short('o')
        .takes_value(true)
        .default_value(r#"beginning"#)
        .long_about(
            r#"Offset to start consuming from:
                     beginning | end | stored |
                     <value>  (absolute offset) |
                     -<value> (relative offset from end)
                     s@<value> (timestamp in ms to start at)
                     e@<value> (timestamp in ms to stop at (not included))"#,
        )
}

pub fn topic() -> Arg<'static> {
    Arg::new("topic")
        .short('t')
        .long("topic")
        .about("Topic")
        .takes_value(true)
}

pub fn brokers() -> Arg<'static> {
    Arg::new("brokers")
        .short('b')
        .long("brokers")
        .about(
            "Broker list. This could also be a cluster name in a config file specified by \
         --config conf.yaml (default ~/.kaf/config)",
        )
        .default_value(BROKERS_DEFAULT)
}

pub fn config() -> Arg<'static> {
    Arg::new("config")
        .short('F')
        .long("config-file")
        .default_value("~/.kaf/config")
        .about(
            "Specify the config file path. The config file should be in yaml/properties format.\
        If it's yaml, the config is used to pick up a cluster, and you have to specify a broker. \
        When it's properties file, there is no need to specify a broker",
        )
}

pub fn extra_config() -> Arg<'static> {
    Arg::new("extra-config")
        .short('X')
        .about(
            "configs that are passed to librdkafka directly. Format: -X key1=val1 -X key2=val2. \
        For copy subcommand: 1) Set different pairs for producer and consumer via: \
        ./kafcat --cp -X key1=val1 <from> -- -X key2=val2 <to>; 2) Set the same pairs for producer \
        and consumer via: ./kafcat -X key1=val1 -X key2=val2 --cp <from> -- <to>",
        )
        .takes_value(true)
        .multiple_values(true)
}

pub fn partition() -> Arg<'static> {
    Arg::new("partition")
        .short('p')
        .long("partition")
        .about("Partition")
        .takes_value(true)
}

pub fn exit() -> Arg<'static> {
    Arg::new("exit")
        .short('e')
        .long("exit")
        .about("Exit successfully when last message received")
}

pub fn format() -> Arg<'static> {
    Arg::new("format")
        .short('s')
        .long("format")
        .about("Serialize/Deserialize format")
        .default_value(FORMAT_DEFAULT)
}

pub fn flush_count() -> Arg<'static> {
    Arg::new("flush-count")
        .long("flush-count")
        .about("Number of messages to receive before flushing stdout")
        .takes_value(true)
}

pub fn flush_bytes() -> Arg<'static> {
    Arg::new("flush-bytes")
        .long("flush-bytes")
        .about("Size of messages in bytes accumulated before flushing to stdout")
        .takes_value(true)
}

pub fn msg_delimiter() -> Arg<'static> {
    Arg::new("msg-delimiter")
        .short('D')
        .about("Delimiter to split input into messages(currently only supports '\\n')")
        .default_value(MSG_DELIMITER_DEFAULT)
}

pub fn key_delimiter() -> Arg<'static> {
    Arg::new("key-delimiter")
        .short('K')
        .about("Delimiter to split input key and message")
        .default_value(KEY_DELIMITER_DEFAULT)
}

pub fn consume_subcommand() -> App<'static> {
    App::new("consume").short_flag('C').args(vec![
        brokers(),
        group_id(),
        topic().required(true),
        partition(),
        offset(),
        exit(),
        msg_delimiter(),
        key_delimiter(),
        format(),
        flush_count(),
        flush_bytes(),
        config(),
        extra_config(),
    ])
}

pub fn produce_subcommand() -> App<'static> {
    App::new("produce").short_flag('P').args(vec![
        brokers(),
        group_id(),
        topic().required(true),
        partition(),
        msg_delimiter(),
        key_delimiter(),
        format(),
        config(),
        extra_config(),
    ])
}

pub fn copy_subcommand() -> App<'static> {
    // this is not meant to be used directly only for help message
    App::new("copy")
        .setting(AppSettings::AllowLeadingHyphen)
        .alias("--cp")
        .arg(Arg::new("from").multiple_values(true).required(true))
        .arg(Arg::new("to").multiple_values(true).last(true).required(true))
        .about(
            "Copy mode accepts two parts of arguments <from> and <to>, the two parts are \
        separated by [--]. <from> is the exact as Consumer mode, and <to> is the exact as Producer \
        mode.",
        )
}

pub fn get_arg_matcher() -> App<'static> {
    App::new("kafcat")
        .version(crate_version!())
        .author("Jiangkun Qiu <qiujiangkun@foxmail.com>")
        .about("cat but kafka")
        .subcommands(vec![
            consume_subcommand(),
            produce_subcommand(),
            copy_subcommand(),
        ])
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(config())
        .arg(extra_config())
        .arg(
            Arg::new("log")
                .long("log")
                .about("Configure level of logging")
                .takes_value(true)
                // See https://docs.rs/env_logger/0.8.3/env_logger/#enabling-logging
                .possible_values(&["error", "warn", "info", "debug", "trace"]),
        )
}

#[derive(Debug, Copy, Clone, EnumString, Display)]
#[strum(serialize_all = "lowercase")]
pub enum WorkingMode {
    Unspecified,
    Consumer,
    Producer,
    Metadata,
    Query,
    Copy,
}

impl WorkingMode {
    pub fn should_have_input_kafka(self) -> bool {
        match self {
            WorkingMode::Unspecified => false,
            WorkingMode::Consumer => true,
            WorkingMode::Producer => false,
            WorkingMode::Metadata => true,
            WorkingMode::Query => true,
            WorkingMode::Copy => true,
        }
    }

    pub fn should_have_output_kafka(self) -> bool {
        match self {
            WorkingMode::Unspecified => false,
            WorkingMode::Consumer => false,
            WorkingMode::Producer => true,
            WorkingMode::Metadata => false,
            WorkingMode::Query => false,
            WorkingMode::Copy => true,
        }
    }
}

impl Default for WorkingMode {
    fn default() -> Self {
        Self::Unspecified
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum KafkaOffset {
    Beginning,
    End,
    Stored,
    Offset(isize),
    OffsetInterval(i64, i64),
    TimeInterval(i64, i64),
}

impl Default for KafkaOffset {
    fn default() -> Self {
        Self::Beginning
    }
}

impl FromStr for KafkaOffset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, String> {
        Ok(match value.to_ascii_lowercase().as_str() {
            "beginning" => KafkaOffset::Beginning,
            "end" => KafkaOffset::End,
            "stored" => KafkaOffset::Stored,
            _ => {
                if let Ok(val) = value.parse() {
                    KafkaOffset::Offset(val)
                } else if let Some(x) = Regex::new(r"s@(\d+)e@(\d+)$").unwrap().captures(value) {
                    let b = x.get(1).unwrap().as_str();
                    let e = x.get(2).unwrap().as_str();
                    KafkaOffset::TimeInterval(b.parse().unwrap(), e.parse().unwrap())
                } else {
                    return Err(format!("Cannot parse {} as offset", value));
                }
            }
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SerdeFormat {
    Text,
    Json,
    Regex(String),
}

impl FromStr for SerdeFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, String> {
        Ok(match value {
            "text" => SerdeFormat::Text,
            "json" => SerdeFormat::Json,
            _ => SerdeFormat::Regex(value.to_owned()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub working_mode: WorkingMode,
    pub consumer_kafka: Option<KafkaConsumerConfig>,
    pub producer_kafka: Option<KafkaProducerConfig>,
    pub log_level: LevelFilter,
}

impl AppConfig {
    pub fn from_args(args: Vec<&str>) -> Self {
        let matches = get_arg_matcher().get_matches_from(args);

        let kafcat_log_env = std::env::var("KAFCAT_LOG").ok();
        let log_level = matches
            .value_of("log")
            .or_else(|| kafcat_log_env.as_deref())
            .map(|x| LevelFilter::from_str(x).expect("Cannot parse log level"))
            .unwrap_or(LevelFilter::Info);

        let clusters_config = ClustersConfig::from_path_yaml(&shellexpand::tilde(
            matches.value_of("config").unwrap(),
        ));

        let mut this = AppConfig {
            working_mode: WorkingMode::Unspecified,
            consumer_kafka: None,
            producer_kafka: None,
            log_level,
        };
        match matches.subcommand() {
            Some(("consume", matches)) => {
                this.working_mode = WorkingMode::Consumer;
                this.consumer_kafka = Some(KafkaConsumerConfig::from_matches(
                    matches,
                    clusters_config.as_ref().ok(),
                ));
            }
            Some(("produce", matches)) => {
                this.working_mode = WorkingMode::Producer;
                this.producer_kafka = Some(KafkaProducerConfig::from_matches(
                    matches,
                    clusters_config.as_ref().ok(),
                ));
            }
            Some(("copy", matches)) => {
                this.working_mode = WorkingMode::Copy;
                let from = vec!["kafka"]
                    .into_iter()
                    .chain(matches.values_of("from").expect("Must specify from"));
                let to = vec!["kafka"]
                    .into_iter()
                    .chain(matches.values_of("to").expect("Must specify to"));

                let consumer = consume_subcommand().get_matches_from(from);
                let producer = produce_subcommand().get_matches_from(to);
                this.consumer_kafka = Some(KafkaConsumerConfig::from_matches(
                    &consumer,
                    clusters_config.as_ref().ok(),
                ));
                this.producer_kafka = Some(KafkaProducerConfig::from_matches(
                    &producer,
                    clusters_config.as_ref().ok(),
                ));
            }
            _ => unreachable!(),
        }

        this
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KafkaConsumerConfig {
    pub auth: KafkaAuthConfig,
    pub group_id: String,
    pub offset: KafkaOffset,
    pub partition: Option<i32>,
    pub topic: String,
    pub exit_on_done: bool,
    pub msg_delim: String,
    pub key_delim: String,
    pub format: SerdeFormat,
    pub msg_count_flush: Option<usize>,
    pub msg_bytes_flush: Option<usize>,
}

impl KafkaConsumerConfig {
    pub fn from_matches(
        matches: &ArgMatches,
        clusters: Option<&ClustersConfig>,
    ) -> KafkaConsumerConfig {
        let brokers = matches
            .value_of("brokers")
            .expect("Must specify brokers")
            .to_owned();
        let group_id = matches.value_of("group-id").unwrap_or("kafcat").to_owned();
        let offset = matches
            .value_of("offset")
            .map(|x| x.parse().expect("Cannot parse offset"))
            .unwrap_or(KafkaOffset::Beginning);
        let partition = matches
            .value_of("partition")
            .map(|x| x.parse().expect("Cannot parse partition"));
        let exit = matches.occurrences_of("exit") > 0;
        let topic = matches
            .value_of("topic")
            .expect("Must specify topic")
            .to_owned();
        let format = matches.value_of("format").expect("Must specify format");
        let msg_delim = matches.value_of("msg-delimiter").unwrap().to_owned();
        let key_delim = matches.value_of("key-delimiter").unwrap().to_owned();
        let msg_count_flush = matches.value_of("flush-count").map(|x| x.parse().unwrap());
        let msg_bytes_flush = matches.value_of("flush-bytes").map(|x| x.parse().unwrap());
        KafkaConsumerConfig {
            auth: ClustersConfig::find_host(clusters, &brokers),
            group_id,
            offset,
            partition,
            topic,
            format: SerdeFormat::from_str(format).unwrap(),
            exit_on_done: exit,
            msg_delim,
            key_delim,
            msg_count_flush,
            msg_bytes_flush,
        }
    }
}

impl Default for KafkaConsumerConfig {
    fn default() -> Self {
        KafkaConsumerConfig {
            auth: KafkaAuthConfig::plaintext(BROKERS_DEFAULT),
            group_id: GROUP_ID_DEFAULT.to_string(),
            offset: KafkaOffset::from_str(OFFSET_DEFAULT).unwrap(),
            partition: None,
            topic: "".to_string(),
            format: SerdeFormat::from_str(FORMAT_DEFAULT).unwrap(),
            exit_on_done: false,
            msg_delim: MSG_DELIMITER_DEFAULT.to_string(),
            key_delim: KEY_DELIMITER_DEFAULT.to_string(),
            msg_count_flush: None,
            msg_bytes_flush: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KafkaProducerConfig {
    pub auth: KafkaAuthConfig,
    pub group_id: String,
    pub partition: Option<i32>,
    pub topic: String,
    pub msg_delim: String,
    pub key_delim: String,
    pub format: SerdeFormat,
}
impl KafkaProducerConfig {
    pub fn from_matches(
        matches: &ArgMatches,
        clusters: Option<&ClustersConfig>,
    ) -> KafkaProducerConfig {
        let brokers = matches
            .value_of("brokers")
            .expect("Must specify brokers")
            .to_owned();
        let group_id = matches.value_of("group-id").unwrap_or("kafcat").to_owned();
        let partition = matches
            .value_of("partition")
            .map(|x| x.parse().expect("Cannot parse partition"));
        let topic = matches
            .value_of("topic")
            .expect("Must specify topic")
            .to_owned();
        let msg_delim = matches.value_of("msg-delimiter").unwrap().to_owned();
        let key_delim = matches.value_of("key-delimiter").unwrap().to_owned();
        let format = matches.value_of("format").expect("Must specify format");
        KafkaProducerConfig {
            auth: ClustersConfig::find_host(clusters, &brokers),
            group_id,
            partition,
            topic,
            msg_delim,
            key_delim,
            format: SerdeFormat::from_str(format).unwrap(),
        }
    }
}

impl Default for KafkaProducerConfig {
    fn default() -> Self {
        KafkaProducerConfig {
            auth: KafkaAuthConfig::plaintext(BROKERS_DEFAULT),
            group_id: GROUP_ID_DEFAULT.to_string(),
            partition: None,
            topic: "".to_string(),
            msg_delim: MSG_DELIMITER_DEFAULT.to_string(),
            key_delim: KEY_DELIMITER_DEFAULT.to_string(),
            format: SerdeFormat::from_str(FORMAT_DEFAULT).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KafkaAuthConfig {
    pub name: String,
    pub brokers: Vec<String>,
    #[serde(rename = "SASL")]
    pub sasl: Option<SaslConfig>,
    #[serde(rename = "TLS")]
    pub tls: Option<TlsConfig>,
    #[serde(rename = "security-protocol")]
    #[serde(default)]
    pub security_protocol: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Copy)]
pub enum SecurityProtocol {
    Plaintext,
    SaslPlaintext,
    Ssl,
    SaslSsl,
}

impl SecurityProtocol {
    pub fn to_string(self) -> &'static str {
        match self {
            SecurityProtocol::Plaintext => "plaintext",
            SecurityProtocol::SaslPlaintext => "sasl_plaintext",
            SecurityProtocol::Ssl => "ssl",
            SecurityProtocol::SaslSsl => "sasl_ssl",
        }
    }
}

impl KafkaAuthConfig {
    pub fn plaintext(host: &str) -> Self {
        Self {
            name: host.to_owned(),
            brokers: host.split_whitespace().map(|x| x.to_owned()).collect(),
            sasl: None,
            tls: None,
            security_protocol: "".to_string(),
            version: "".to_string(),
        }
    }
    pub fn get_security_protocol(&self) -> SecurityProtocol {
        match (&self.sasl, &self.tls) {
            (None, None) => SecurityProtocol::Plaintext,
            (Some(_sasl), None) => SecurityProtocol::SaslPlaintext,
            (None, Some(_tls)) => SecurityProtocol::Ssl,
            (Some(_sasl), Some(_tls)) => SecurityProtocol::SaslSsl,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ClustersConfig {
    pub clusters: Vec<KafkaAuthConfig>,
}
impl ClustersConfig {
    fn from_path_yaml(path: &str) -> std::io::Result<Self> {
        let file = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&file).expect(&format!("Cannot parse config file: {}", path)))
    }

    fn from_str_yaml(yaml: &str) -> serde_yaml::Result<Self> {
        serde_yaml::from_str(&yaml)
    }
    fn find_host(this: Option<&Self>, host: &str) -> KafkaAuthConfig {
        match this {
            Some(this) => this
                .clusters
                .iter()
                .find(|x| x.name == host)
                .map(|x| x.clone())
                .unwrap_or_else(|| KafkaAuthConfig::plaintext(host)),
            None => KafkaAuthConfig::plaintext(host),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TlsConfig {
    #[serde(default)]
    pub cafile: String,
    #[serde(default)]
    pub clientfile: String,
    #[serde(default)]
    pub clientkeyfile: String,
    #[serde(default)]
    pub insecure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SaslConfig {
    mechanism: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(rename = "clientID")]
    #[serde(default)]
    client_id: String,
    #[serde(rename = "clientSecret")]
    #[serde(default)]
    client_secret: String,
    #[serde(rename = "tokenURL")]
    #[serde(default)]
    token_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumer_config() {
        let config =
            AppConfig::from_args(vec!["kafcat", "-C", "-b", "localhost", "-t", "topic", "-e"]);
        assert_eq!(
            config.consumer_kafka.unwrap(),
            KafkaConsumerConfig {
                auth: KafkaAuthConfig::plaintext("localhost"),
                group_id: "kafcat".to_string(),
                offset: KafkaOffset::Beginning,
                partition: None,
                topic: "topic".to_string(),
                exit_on_done: true,
                ..Default::default()
            }
        )
    }
    #[test]
    fn producer_config() {
        let config = AppConfig::from_args(vec!["kafcat", "-P", "-b", "localhost", "-t", "topic"]);
        assert_eq!(
            config.producer_kafka.unwrap(),
            KafkaProducerConfig {
                auth: KafkaAuthConfig::plaintext("localhost"),
                group_id: "kafcat".to_string(),
                partition: None,
                topic: "topic".to_string(),
                ..Default::default()
            }
        )
    }
    #[test]
    fn copy_config() {
        let config = AppConfig::from_args(vec![
            "kafcat",
            "copy",
            "-b",
            "localhost1",
            "-t",
            "topic1",
            "-e",
            "--",
            "-b",
            "localhost2",
            "-t",
            "topic2",
        ]);
        assert_eq!(
            config.consumer_kafka.unwrap(),
            KafkaConsumerConfig {
                auth: KafkaAuthConfig::plaintext("localhost1"),
                group_id: "kafcat".to_string(),
                offset: KafkaOffset::Beginning,
                partition: None,
                topic: "topic1".to_string(),
                exit_on_done: true,
                ..Default::default()
            }
        );
        assert_eq!(
            config.producer_kafka.unwrap(),
            KafkaProducerConfig {
                auth: KafkaAuthConfig::plaintext("localhost2"),
                group_id: "kafcat".to_string(),
                partition: None,
                topic: "topic2".to_string(),
                ..Default::default()
            }
        );
    }
    macro_rules! test_read_clusters_config {
        ($fn_name: ident, $name: ident) => {
            #[test]
            fn $fn_name() {
                let config = include_str!(concat!("../examples/", stringify!($name), ".yaml"));
                let config = ClustersConfig::from_str_yaml(config).expect("Cannot parse config");
                drop(config);
            }
        };
    }
    test_read_clusters_config!(test_read_config_basic, basic);
    test_read_clusters_config!(test_read_config_sasl_plaintext, sasl_plaintext);
    test_read_clusters_config!(test_read_config_sasl_ssl, sasl_ssl);
    test_read_clusters_config!(test_read_config_sasl_ssl_custom_ca, sasl_ssl_custom_ca);
    test_read_clusters_config!(test_read_config_sasl_ssl_insecure, sasl_ssl_insecure);
    test_read_clusters_config!(test_read_config_sasl_ssl_scram, sasl_ssl_scram);
    test_read_clusters_config!(test_read_config_ssl_keys, ssl_keys);
}
