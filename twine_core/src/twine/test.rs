#![allow(unused_imports)]
use super::*;

// tests
#[cfg(test)]
mod test {

  use super::*;

  const STRANDJSON: &'static str = r#"
    {
      "cid": {
        "/": "bafyriqe3zxf5g4ifgqhea5zozxpdcfi5qcpkfpogtxzbizmmuxdjuzuq44a2cifbr7xplo4kcfsdz2c5pxfxektavrqxxb3nvbmclxz7qiz6e"
      },
      "data": {
        "content": {
          "key": {
            "alg": "RS256",
            "e": "AQAB",
            "kty": "RSA",
            "n": "zI7ywpS55pGdNZ3NwaWmFNVnYMeaxwNdAtfc8nTewwvkKJ4LE1wzYcWXebZjt_D9NtoB2BS9Lo_HYSIwfsIdTLymCdEn9iJvBANRU6ZjO_OeOIFTeCzBb-nZ7_XFXLUl8Xv2GGYFl1yZoKwWVwypcfWVKKDsUz9OxXKWZ4sq9ACwrLjY-w9U_EgqTbRSZvfZQOk1c6CbORjXNRaoVCgEU6_jzgHzWMMiDZIgTf_lRWy5vIiJJV-fd0c0XAJpAZjO1ZqzwaBMUe64KLjcLNxIV2VdeOrJbiis9s8QGVGZAYw40sk74B-OMssrftXD-_cRORR8FP4FMAaybuyQvDB8w0pqw5lHOZ3_2WkmS8tDm6X_CKFxBI6ZzO3Z4m8yEaSTK2-YrWchWlmQ4ADiGdpGCymoowEnv366zi86_Plqqla8e8vcCLkq9KGMOICVZsL4juvptOD_wEdLYBiHrSL8kLCyK7fJj2dT7eJ1S5H2UJ_SaI1jb5Y0zTY0fgHfatzmc2ZG8T0tobaC_1RtM4Y5bzm7eMqXt3S0vFlXdZhySw1_2bxW-rA1WcM2PUiIYqvaXtrHbAXDJCvZ_pLUdi98JA1TCzUuemKwu3kbROuwNiakev8vq7NDWBipo_cIOYs4GaXb3FhElzC7W4F22jHiNI_uT_wERSlQhzXnSxqIRYc"
          },
          "links_radix": 32,
          "meta": {
            "description": "Experiment status information",
            "genesis": "2023-10-26T17:38:30.848Z",
            "name": "NIST-bell-experiment"
          },
          "mixins": [

          ],
          "source": "random.colorado.edu",
          "specification": "twine/1.0.x/bell/1.0.x"
        },
        "signature": "eyJhbGciOiJSUzI1NiJ9.FEARpnMB4MWvXMDIzMlU7DagYYKyLzHI7-TSj8Oq-VdrSCVS5PFty6U5QdkFWl3zp65R3aE54FTNkqsO2maRnC8K.H39yBRvIygMLalQIPQo84sypZuV5EnUvS5dplfRSC4Fk4Isy1xqIKcoAKT8FO9liDxZXelx27n9-e7tYivdq35fHnDWC3aH6dv72S_5Iy2e0zRJ25D-kHZoOOctLknbTAG6i-nLJZOrnfzHAIi8XOnK83TyMfIfBbnbUJvUMYvtYkYvRtkonAVvkFhWPg5o7ZFnbhu1XsWvY92PRQ-xzUbo0BY6nxNT7l6GAWKzyHeSCFrLFIC5AR4tEZ75lvWQnfTeUDcNds4w-SD7RmDZJ-3aBqSLS89uZoxh7UVKRWPxz80XJD3iveE2Kv3qLN9iqGFFPoActrECYusEUdbn12dHS2zgKLyDbOpxZFuSPI2DjFUmYLZpWK5WnuTI9KmE9t8dFtNZT-HB02C9iuO0K24AB5i59nq3TSpRTUjQMRBfK45N0tWN5wTukonKAxjxxZl-IBwka1fhA85C34XIsb7OuIp4OH9p_nUgchHg_jMc3IiwIRovlytd_l4fmxkKdmzr5qVl0B6xUJHXydIo77BCvDbv7OW3Hxrz9r73EidwINzH_yrAKyb4xV0xHKo_nHoFdpAd5sW7cxsF_USrfA5axuyGYRrsS_RjdhkDLseKRDxtYkM0nIuLwZogG6a5HJ1729kJiSEkNSrFhfYFmBh7hpLLAkzRqqc1bqha4xIQ"
      }
    }"#;

  const TIXELJSON: &'static str = r#"
    {
      "cid": {
        "/": "bafyriqgafbhhudahpnzrdvuzjjczro43i4mnv7637vq4oh6m6lfdccpazmmurfu4vluy7iddrhwbbfvjs62uo2wrzx4axaxx5lv7pfmqveqt2"
      },
      "data": {
        "content": {
          "chain": {
            "/": "bafyriqe3zxf5g4ifgqhea5zozxpdcfi5qcpkfpogtxzbizmmuxdjuzuq44a2cifbr7xplo4kcfsdz2c5pxfxektavrqxxb3nvbmclxz7qiz6e"
          },
          "index": 100,
          "links": [
            {
              "/": "bafyriqgqnqyqrjpq54oy5zv4w3ev4zm36rjuhbmmvw2noaqeii2ru3azvm6tc7qcknrdzegh44rbszdd3lr6m5cwnl7eohzubx2uolhqab2qm"
            },
            {
              "/": "bafyriqdibseq7reosvlqhnhw6eudtcv2e2nzqdsmvnp3w43wybriiqkqfbbwhw4sysddgtfzlhv4tzv264snd3lmy4hqz64qva4kjjlptr6re"
            }
          ],
          "mixins": [
            {
              "chain": {
                "/": "bafyriqhijw5soalbtppuwwjbskfiriqy6swa5qd5trgbzx4nypnu7mp7iyhx7m2iin2nxnglmngvjgirzb7bvlisxq3uygoik7ozbybxnftiw"
              },
              "value": {
                "/": "bafyriqgrnibxorlgvxtbinfbbo5preifuvjq3p7lnvg6wqrwq43nfag5pgc5p7x5yblu2asgmhl65wefgqxrr7trsx6otikmy3s47rsrrwo6q"
              }
            }
          ],
          "payload": {
            "dataHash": "a5e3298e68cb7e2f904d973c2a7cc690044252bfc803cc260e8e7c0365c8681b1e00cb8adc30c325c2ce33ffde718a3c5d87f7067b4927b49eb7a5fd6205804f",
            "ref": {
              "/": "bafyriqgrnibxorlgvxtbinfbbo5preifuvjq3p7lnvg6wqrwq43nfag5pgc5p7x5yblu2asgmhl65wefgqxrr7trsx6otikmy3s47rsrrwo6q"
            },
            "spacelikeSeparation": {
              "alice": {
                "margin": 32.3,
                "uncertainty": 3.5
              },
              "bob": {
                "margin": 50.1,
                "uncertainty": 3.6
              },
              "units": "ns"
            },
            "status": 0,
            "timestamp": "2023-10-26T21:25:56.936Z",
            "type": "result"
          },
          "source": "random.colorado.edu"
        },
        "signature": "eyJhbGciOiJSUzI1NiJ9.FEAnDYJjM3kQ-woX_46Bp5mRTlAzNqZPqx3jyOqmB9W_jR2cHxZT-Hc8o16E4eUotiquqHiYyL__Ck4JHKQWz65f.ZXWAko1_jvapXljIDAGxNDj0WiwA4pb5CXM7GrpvuJbEhg-sFlPCVYegvFAQqvIa-Bqyv6CsDxISh73vJAXzRFqoJy4Yt716yRsJ-bZsL-L5r9gT8xYMnlAW0F7Q8Rm2_fL38gdoKZI8BMfn87AlR-DaTtEivwocE5hlBIwo2DSsiQCBa-9Q4cnlimhM8nzbTMFZc-QdHR1Y2lvm7NAqXFa72OzizQOICNTY0Ff0eY4HXDopv6f_9c-bkhVA3VchpkH7hilximBdqP7KW_9G1byGmxXWdyTyi3C0Jujvnf7sl0Mx9kaD8Jx7xRyFGCsGISgd4D90uSjgvqOHgUkRAm4KyK9tHZS1zX0FoiuRax2D7EY6caV7JPtu0GFyVv6LlsdJ81Q6kj6_aDHUJ54cz4YaZbQ_qbobkdp0Yo-WnxC3c_SwgkJyujD3qlWxpAcvriafY6VNI_U9lX-xvfuhe5b1wZam03oHc-dZau1fFYIHRU1Zweo9-l0wJ3btEPRbERAXIAWbw99YLqy0-BDoomTCc6PBL0e8D5FNy0etO4ThI0cjybz_KKsyd2Smk3aWBJ6PsrdDCXWEwcJt91cneKXb5keG1tQCpcDsTjjS5suAwu4xJk49Xi7fkmqr3LE8TtZKgXeMl2nOJem7BT90ZbN78ZKThB52JXsKRAyevys"
      }
    }
  "#;

  #[test]
  fn test_deserialize_tixel_json() {
    let res = Tixel::from_dag_json(TIXELJSON);
    dbg!(&res);
    assert!(res.is_ok(), "Failed to deserialize Tixel: {:?}", res.err());
  }

  #[test]
  fn test_deserialize_strand_json(){
    let res = Strand::from_dag_json(STRANDJSON);
    dbg!(&res);
    assert!(res.is_ok(), "Failed to deserialize Strand: {:?}", res.err());
    // println!("{}", std::str::from_utf8(&DagJsonCodec::encode_to_vec(&res.unwrap()).unwrap()).unwrap());
    // print hex
    let cbor = res.unwrap().to_bytes();
    for byte in cbor {
      print!("{:02x}", byte);
    }
  }

  #[test]
  fn test_deserialize_tixel_bytes(){
    let tixel = Tixel::from_dag_json(TIXELJSON).unwrap();
    let bytes = tixel.to_bytes();
    let res = Tixel::from_block(tixel.cid(), bytes);
    dbg!(&res);
    assert!(res.is_ok(), "Failed to deserialize Tixel from bytes: {:?}", res.err());
  }

  #[test]
  fn test_deserialize_strand_bytes(){
    let strand = Strand::from_dag_json(STRANDJSON).unwrap();
    let res = Strand::from_block(strand.cid(), strand.to_bytes());
    // dbg!(&res);
    assert!(res.is_ok(), "Failed to deserialize Strand from bytes: {:?}", res.err());
  }

  #[test]
  fn test_deserialize_arbitrary() {
    let twine = Twine::from_dag_json(STRANDJSON);
    assert!(twine.is_ok(), "Failed to deserialize Strand: {:?}", twine.err());
    assert!(twine.unwrap().is_strand(), "Twine is not a Strand");
  }

  #[test]
  fn test_in_out_json(){
    let twine = Twine::from_dag_json(TIXELJSON).unwrap();
    let json = twine.to_dag_json();
    let twine2 = Twine::from_dag_json(&json).unwrap();
    assert_eq!(twine, twine2, "Twine JSON roundtrip failed. Json: {}", json);
    assert!(twine2.is_tixel(), "Twine is not a Tixel");
  }
}
