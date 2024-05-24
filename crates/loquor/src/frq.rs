use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Frq {
    pub hop_size: i32,
    pub average_f0: f64,
    pub length: i32,
    pub f0: Vec<f64>,
    pub amp: Vec<f64>,
}

// [header: FREQ0003][hop_size: i32][average_f0: f64][empty: 16 bytes][length: i32][[f0: f64][amp: f64] * length]

impl Frq {
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Err(anyhow::anyhow!("Data is too short"));
        }
        if &data[0..8] != "FREQ0003".as_bytes() {
            return Err(anyhow::anyhow!("Invalid header: {:?}", &data[0..8]));
        }
        let hop_size = i32::from_le_bytes(data[8..12].try_into().unwrap());
        let average_f0 = f64::from_le_bytes(data[12..20].try_into().unwrap());
        let length = i32::from_le_bytes(data[36..40].try_into().unwrap());
        let mut f0 = Vec::new();
        let mut amp = Vec::new();
        for i in 0..length {
            f0.push(f64::from_le_bytes(
                data[((40 + i * 16) as usize)..(48 + i * 16) as usize]
                    .try_into()
                    .unwrap(),
            ));
            amp.push(f64::from_le_bytes(
                data[((48 + i * 16) as usize)..(56 + i * 16) as usize]
                    .try_into()
                    .unwrap(),
            ));
        }
        Ok(Self {
            hop_size,
            average_f0,
            length,
            f0,
            amp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let frq = include_bytes!("./test.frq");
        Frq::parse(frq).unwrap();
    }
}
