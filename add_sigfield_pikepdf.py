#!/usr/bin/env python3
import sys
from pathlib import Path
import pikepdf

# 署名フィールドの追加
def add_signature_field_with_pikepdf(input_pdf: str, output_pdf: str):
    pdf = pikepdf.open(input_pdf)
    
    first_page = pdf.pages[0]
    
    sig_field = pdf.make_indirect(pikepdf.Dictionary({
        '/FT': pikepdf.Name('/Sig'),
        '/T': pikepdf.String('KyogakuDendoSignature'),
    }))
    
    widget = pdf.make_indirect(pikepdf.Dictionary({
        '/Type': pikepdf.Name('/Annot'),
        '/Subtype': pikepdf.Name('/Widget'),
        '/Rect': pikepdf.Array([0, 0, 100, 50]),
        '/P': first_page.obj,
        '/Parent': sig_field,
        '/F': 132,
    }))
    
    sig_field['/Kids'] = pikepdf.Array([widget])
    
    if '/Annots' not in first_page:
        first_page['/Annots'] = pikepdf.Array()
    first_page['/Annots'].append(widget)
    
    acro_form = pdf.make_indirect(pikepdf.Dictionary({
        '/Fields': pikepdf.Array([sig_field]),
        '/SigFlags': 3
    }))
    
    pdf.Root['/AcroForm'] = acro_form
    
    pdf.save(output_pdf)
    pdf.close()
    
    print(f"✓ Added signature field with /Kids structure: {output_pdf}")

if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <input_pdf> <output_pdf>")
        sys.exit(1)
    
    input_pdf = sys.argv[1]
    output_pdf = sys.argv[2]
    
    if not Path(input_pdf).exists():
        print(f"Error: Input PDF not found: {input_pdf}")
        sys.exit(1)
    
    try:
        add_signature_field_with_pikepdf(input_pdf, output_pdf)
    except Exception as e:
        print(f"Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
