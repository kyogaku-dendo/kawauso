#!/usr/bin/env python3
import sys
from pathlib import Path
from pyhanko.sign import signers
from pyhanko.pdf_utils.incremental_writer import IncrementalPdfFileWriter

def sign_pdf_with_pyhanko(
    input_pdf: str,
    output_pdf: str,
    cert_file: str,
    key_file: str,
    signature_name: str,
    uuid: str,
    reason: str = "Digital signature",
):
    signer_obj = signers.SimpleSigner.load(
        cert_file=cert_file,
        key_file=key_file,
        ca_chain_files=None,
        key_passphrase=None,
    )
    
    # PDFを開く
    with open(input_pdf, 'rb') as inf:
        w = IncrementalPdfFileWriter(inf)
        
        meta = signers.PdfSignatureMetadata(
            field_name=signature_name,
            reason=f"{reason} - ID: {uuid}", # reasonに一意なUUIDを含める
            location=None,
        )
        
        pdf_signer = signers.PdfSigner(
            meta,
            signer=signer_obj,
            stamp_style=None,
        )
        
        # 署名を追加
        out = pdf_signer.sign_pdf(
            w,
            existing_fields_only=True,
        )
        
        with open(output_pdf, 'wb') as outf:
            outf.write(out.getbuffer())
    
    print(f"PDF signed successfully: {output_pdf}")

if __name__ == '__main__':
    if len(sys.argv) != 7:
        print(f"Usage: {sys.argv[0]} <input_pdf> <output_pdf> <cert_file> <key_file> <signature_name> <uuid>")
        print(f"Example: {sys.argv[0]} input.pdf output.pdf cert.crt key.pem KyogakuDendoSignature 123e4567-e89b-12d3-a456-426614174000")
        sys.exit(1)
    
    input_pdf = sys.argv[1]
    output_pdf = sys.argv[2]
    cert_file = sys.argv[3]
    key_file = sys.argv[4]
    signature_name = sys.argv[5]
    uuid = sys.argv[6]
    
    for file_path, name in [(input_pdf, "Input PDF"), (cert_file, "Certificate"), (key_file, "Key")]:
        if not Path(file_path).exists():
            print(f"Error: {name} not found: {file_path}")
            sys.exit(1)
    
    try:
        sign_pdf_with_pyhanko(
            input_pdf=input_pdf,
            output_pdf=output_pdf,
            cert_file=cert_file,
            key_file=key_file,
            signature_name=signature_name,
            uuid=uuid,
        )
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
