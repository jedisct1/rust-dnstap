fn main() {
    protobuf_codegen_pure::Codegen::new()
        .out_dir("src")
        .inputs(["src/dnstap_pb.proto"])
        .include("src")
        .customize(protobuf_codegen_pure::Customize {
            ..Default::default()
        })
        .run()
        .expect("protoc");
}
