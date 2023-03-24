use libipld::{ipld, Ipld};


fn main() {
    // the ipld macro makes it easy to generate complex data shapes
    let payload: HashMap<String, Ipld> = ipld!({
        "key": "value",
        "key": [1,2,3],
    }).into(); // ...and convert `Ipld::map` to 

    chain
}