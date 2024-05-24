extern crate worldline;
use regex_macro::regex;
use std::io::Read;
use tyc_utau::CVC_B3_ROOT;
use worldline::SynthRequest;

#[test]
fn test_synth() {
    let mut synth = worldline::PhraseSynth::new();

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .init();

    let file = std::fs::File::open(format!("{}/_ああR.wav", CVC_B3_ROOT)).unwrap();
    let (header, data) = wav_io::read_from_file(file).unwrap();

    let mut frq_file = std::fs::File::open(format!("{}/_ああR_wav.frq", CVC_B3_ROOT)).unwrap();
    let mut frq = Vec::new();
    frq_file.read_to_end(&mut frq).unwrap();

    let mut oto_ini = Vec::new();
    let mut oto_ini_file = std::fs::File::open(format!("{}/oto.ini", CVC_B3_ROOT)).unwrap();
    oto_ini_file.read_to_end(&mut oto_ini).unwrap();
    let oto_ini = encoding_rs::SHIFT_JIS.decode(&oto_ini).0;

    let a_oto = oto_ini.lines().next().unwrap();
    dbg!(a_oto);

    // _ああR.wav=- あ2_B3,149.905,171.608,-866.658,46.608,0.0
    let pattern = regex!("(?P<name>.+)=(?P<alias>.+),(?P<offset>.+),(?P<consonant>.+),(?P<cut_off>.+),(?P<preutter>.+),(?P<overlap>.+)");
    let captures = pattern.captures(a_oto).unwrap();
    dbg!(&captures);

    let req = SynthRequest {
        sample_fs: header.sample_rate as i32,
        sample: data.into_iter().map(|x| x as f64).collect(),
        frq,
        tone: 40,
        con_vel: 100.0,
        offset: captures["offset"].parse().unwrap(),
        required_length: 1000.0,
        consonant: captures["consonant"].parse().unwrap(),
        cut_off: captures["cut_off"].parse().unwrap(),
        volume: 100.0,
        modulation: 0.0,
        tempo: 0.0,
        pitch_bend: vec![0],
        flag_g: 0,
        flag_o: 0,
        flag_p: 0,
        flag_mt: 0,
        flag_mb: 0,
        flag_mv: 0,
    };

    synth.add_request(&req, 0.0, 0.0, 900.0, 5.0, 35.0);

    synth.set_curves(
        &vec![261.0f64; 100],
        &vec![0.5f64; 100],
        &vec![0.5f64; 100],
        &vec![0.5f64; 100],
        &vec![0.5f64; 100],
    );

    let data = synth.synth();

    let mut file = std::fs::File::create("test.wav").unwrap();
    wav_io::write_to_file(&mut file, &header, &data).unwrap();
}
