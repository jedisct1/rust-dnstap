fn main() {
    protobuf_codegen::Codegen::new()
        .pure()
        .out_dir("src")
        .inputs(["src/dnstap_pb.proto"])
        .include("src")
        .customize(protobuf_codegen::Customize::default())
        .run()
        .expect("protoc");
}
