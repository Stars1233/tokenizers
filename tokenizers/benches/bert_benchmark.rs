#[macro_use]
extern crate criterion;

mod common;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use criterion::{Criterion, Throughput};
use tokenizers::models::wordpiece::{WordPiece, WordPieceTrainerBuilder};
use tokenizers::normalizers::{BertNormalizer, NormalizerWrapper};
use tokenizers::pre_tokenizers::bert::BertPreTokenizer;
use tokenizers::processors::bert::BertProcessing;
use tokenizers::{decoders, EncodeInput, Model, TokenizerImpl};

use common::{iter_bench_encode, iter_bench_encode_batch, iter_bench_train};
use tokenizers::decoders::DecoderWrapper;
use tokenizers::pre_tokenizers::whitespace::Whitespace;
use tokenizers::processors::PostProcessorWrapper;

static BATCH_SIZE: usize = 1_000;

type BertTokenizer = TokenizerImpl<
    WordPiece,
    BertNormalizer,
    BertPreTokenizer,
    BertProcessing,
    decoders::wordpiece::WordPiece,
>;

/// Resembling the BertTokenizer implementation from the Python bindings.
fn create_bert_tokenizer(wp: WordPiece) -> BertTokenizer {
    let sep_id = *wp.get_vocab().get("[SEP]").unwrap();
    let cls_id = *wp.get_vocab().get("[CLS]").unwrap();
    let mut tokenizer = TokenizerImpl::new(wp);
    tokenizer.with_pre_tokenizer(Some(BertPreTokenizer));
    tokenizer.with_normalizer(Some(BertNormalizer::default()));
    tokenizer.with_decoder(Some(decoders::wordpiece::WordPiece::default()));
    tokenizer.with_post_processor(Some(BertProcessing::new(
        ("[SEP]".to_string(), sep_id),
        ("[CLS]".to_string(), cls_id),
    )));
    tokenizer
}

pub fn bench_bert(c: &mut Criterion) {
    let wp = WordPiece::from_file("data/bert-base-uncased-vocab.txt")
        .build()
        .unwrap();
    let tokenizer = create_bert_tokenizer(wp);
    let mut group = c.benchmark_group("bert-encode");
    let data = std::fs::read_to_string("data/big.txt").unwrap();
    group.throughput(Throughput::Bytes(data.len() as u64));
    let mut lines: Vec<EncodeInput> = vec![];
    let mut batches: Vec<Vec<EncodeInput>> = vec![vec![]];
    for line in BufReader::new(File::open(Path::new("data/big.txt")).unwrap()).lines() {
        let line: EncodeInput = line.unwrap().into();
        lines.push(line.clone());
        if batches.last().unwrap().len() >= BATCH_SIZE {
            batches.push(vec![]);
        }
        batches.last_mut().unwrap().push(line);
    }

    group.bench_function("WordPiece BERT encode", |b| {
        b.iter_custom(|iters| iter_bench_encode(iters, &tokenizer, &lines))
    });

    group.bench_function("WordPiece BERT encode batch", |b| {
        b.iter_custom(|iters| iter_bench_encode_batch(iters, &tokenizer, &batches))
    });
}

fn bench_train_small(c: &mut Criterion) {
    let mut trainer = WordPieceTrainerBuilder::default()
        .show_progress(false)
        .build();
    type Tok = TokenizerImpl<
        WordPiece,
        NormalizerWrapper,
        Whitespace,
        PostProcessorWrapper,
        DecoderWrapper,
    >;
    let mut tokenizer = Tok::new(WordPiece::default());
    tokenizer.with_pre_tokenizer(Some(Whitespace {}));
    let mut group = c.benchmark_group("bert-train-small");
    let data = std::fs::read_to_string("data/small.txt").unwrap();
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("WordPiece Train vocabulary (small)", |b| {
        b.iter_custom(|iters| {
            iter_bench_train(
                iters,
                &mut tokenizer,
                &mut trainer,
                vec!["data/small.txt".to_string()],
            )
        })
    });
}

fn bench_train_big(c: &mut Criterion) {
    let mut trainer = WordPieceTrainerBuilder::default()
        .show_progress(false)
        .build();
    type Tok = TokenizerImpl<
        WordPiece,
        NormalizerWrapper,
        Whitespace,
        PostProcessorWrapper,
        DecoderWrapper,
    >;
    let mut tokenizer = Tok::new(WordPiece::default());
    tokenizer.with_pre_tokenizer(Some(Whitespace {}));
    let mut group = c.benchmark_group("bert-train-big");
    let data = std::fs::read_to_string("data/big.txt").unwrap();
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function("WordPiece Train vocabulary (big)", |b| {
        b.iter_custom(|iters| {
            iter_bench_train(
                iters,
                &mut tokenizer,
                &mut trainer,
                vec!["data/big.txt".to_string()],
            )
        })
    });
}

criterion_group! {
    name = bert_benches;
    config = Criterion::default().sample_size(20);
    targets = bench_bert
}

criterion_group! {
    name = benches_train_small;
    config = Criterion::default().sample_size(10);
    targets = bench_train_small
}

criterion_group! {
    name = benches_train_big;
    config = Criterion::default().sample_size(10);
    targets = bench_train_big
}

criterion_main!(bert_benches, benches_train_small, benches_train_big);
