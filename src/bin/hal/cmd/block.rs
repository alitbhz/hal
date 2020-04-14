use std::io::Write;

use bitcoin::consensus::encode::{deserialize, serialize};
use bitcoin::{Block, BlockHeader};

use cmd;
use cmd::tx::create_transaction;
use hal::block::{BlockHeaderInfo, BlockInfo};

pub fn subcommand<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand_group("block", "manipulate blocks")
		.subcommand(cmd_create())
		.subcommand(cmd_decode())
}

pub fn execute<'a>(matches: &clap::ArgMatches<'a>) {
	match matches.subcommand() {
		("create", Some(ref m)) => exec_create(&m),
		("decode", Some(ref m)) => exec_decode(&m),
		(_, _) => unreachable!("clap prints help"),
	};
}

fn cmd_create<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("create", "create a raw block from JSON").args(&[
		cmd::arg("block-info", "the block info in JSON").required(true),
		cmd::opt("raw-stdout", "output the raw bytes of the result to stdout")
			.short("r")
			.required(false),
	]).long_about(r#"
Create a block from JSON. Use the same format as the `hal block decode` output.

It's possible to pass the JSON string as the first argument or pass it via stdin.

One can chose to pass in transaction info objects like used for `hal tx create` into the
"transactions" field or hexadecimal raw transactions in the "raw_transactions" field.

Example format:
{
	"header": {
		"block_hash": "0000000000000000000b52e5f79972ccb1b42f14bd5097381551ed7bb4f78478",
		"version": 1,
		"previous_block_hash": "00000000000000000011604b6d32a6074720f60aae04edf20396bbaf65e50edc",
		"merkle_root": "b7f360ae3bb1f6ca1935269d8955808c6606ff20af9b98fbc3ddb7de6c9df5c3",
		"time": 12345,
		"bits": 12345,
		"nonce": 12345
	},

	!! only either of these fields should be provided !!
	"transactions": [
		{ ... }
	],
	"raw_transaction": [
		"deadbeef"
	]
}"#
	)
}

fn create_block_header(info: BlockHeaderInfo) -> BlockHeader {
	if info.block_hash.is_some() {
		warn!("Field \"block_hash\" is ignored.");
	}

	BlockHeader {
		version: info.version,
		prev_blockhash: info.previous_block_hash,
		merkle_root: info.merkle_root,
		time: info.time,
		bits: info.bits,
		nonce: info.nonce,
	}
}

fn exec_create<'a>(matches: &clap::ArgMatches<'a>) {
	let json_block = matches.value_of("block-info").expect("no JSON blok info provided");
	let info: BlockInfo = serde_json::from_str(json_block).expect("invalid JSON");

	if info.txids.is_some() {
		warn!("Field \"txids\" is ignored.");
	}

	let block = Block {
		header: create_block_header(info.header),
		txdata: match (info.transactions, info.raw_transactions) {
			(Some(_), Some(_)) => panic!("Can't provide transactions both in JSON and raw."),
			(None, None) => panic!("No transactions provided."),
			(Some(infos), None) => infos.into_iter().map(create_transaction).collect(),
			(None, Some(raws)) => raws
				.into_iter()
				.map(|r| deserialize(&r.0).expect("invalid raw transaction"))
				.collect(),
		},
	};

	let block_bytes = serialize(&block);
	if matches.is_present("raw-stdout") {
		::std::io::stdout().write_all(&block_bytes).unwrap();
	} else {
		print!("{}", hex::encode(&block_bytes));
	}
}

fn cmd_decode<'a>() -> clap::App<'a, 'a> {
	cmd::subcommand("decode", "decode a raw block to JSON").args(&cmd::opts_networks()).args(&[
		cmd::opt_yaml(),
		cmd::arg("raw-block", "the raw block in hex").required(true),
		cmd::opt("txids", "provide transactions IDs instead of full transactions"),
	])
}

fn exec_decode<'a>(matches: &clap::ArgMatches<'a>) {
	let hex_tx = matches.value_of("raw-block").expect("no raw block provided");
	let raw_tx = hex::decode(hex_tx).expect("could not decode raw block hex");
	let block: Block = deserialize(&raw_tx).expect("invalid block format");

	if matches.is_present("txids") {
		let info = hal::block::BlockInfo {
			header: hal::GetInfo::get_info(&block.header, cmd::network(matches)),
			txids: Some(block.txdata.iter().map(|t| t.txid()).collect()),
			transactions: None,
			raw_transactions: None,
		};
		cmd::print_output(matches, &info)
	} else {
		let info = hal::GetInfo::get_info(&block, cmd::network(matches));
		cmd::print_output(matches, &info)
	}
}
