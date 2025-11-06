import os
from PIL import Image
import pytesseract

def process_image(image_path, output_dir):
    img = Image.open(image_path)
    img_width, img_height = img.size

    # レイアウト抽出
    data = pytesseract.image_to_data(img, lang='jpn', output_type=pytesseract.Output.DICT)
    n_boxes = len(data['text'])
    lines = {}
    # 各行ごとにバウンディングボックスをまとめる
    for i in range(n_boxes):
        if int(data['conf'][i]) > 80 and data['text'][i].strip() != '':
            line_num = data['line_num'][i]
            if line_num not in lines:
                lines[line_num] = {
                    'left': data['left'][i],
                    'top': data['top'][i],
                    'right': data['left'][i] + data['width'][i],
                    'bottom': data['top'][i] + data['height'][i]
                }
            else:
                lines[line_num]['left'] = min(lines[line_num]['left'], data['left'][i])
                lines[line_num]['top'] = min(lines[line_num]['top'], data['top'][i])
                lines[line_num]['right'] = max(lines[line_num]['right'], data['left'][i] + data['width'][i])
                lines[line_num]['bottom'] = max(lines[line_num]['bottom'], data['top'][i] + data['height'][i])

    # 行ごとに切り出して保存
    basename = os.path.splitext(os.path.basename(image_path))[0]
    subdir = os.path.join(output_dir, basename)
    os.makedirs(subdir, exist_ok=True)

    for i, box in lines.items():
        left = 0  # 画像全幅を使用
        right = img_width
        top = box['top']
        bottom = box['bottom']
        cropimg = img.crop((left, top, right, bottom))
        cropimg.save(os.path.join(subdir, f"line_{i}.png"))

def process_directory(src_dir, output_dir):
    for fname in os.listdir(src_dir):
        if fname.lower().endswith(('.png', '.jpg', '.jpeg')):
            process_image(os.path.join(src_dir, fname), output_dir)

# process_directory('input_images', 'output_dir')
