#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
微博 HTML → Markdown 转换器
将 @本恰恰-_原创微博.html 中的每条微博转换为独立的 Markdown 文件。

目录结构：weibo/YYYY/MM/YYYY-MM-DD-正文前10字.md
图片统一复制到：images/ 文件夹
"""

import re
import os
import shutil
import html
from pathlib import Path
from urllib.parse import unquote, urlparse, parse_qs

# ─── 路径配置 ────────────────────────────────────────────────────────────────
BASE_DIR = Path(__file__).parent
HTML_FILE = BASE_DIR / "@本恰恰-_原创微博.html"
FILES_DIR = BASE_DIR / "@本恰恰-_原创微博_files"
OUTPUT_DIR = BASE_DIR / "weibo"
IMAGES_DIR = OUTPUT_DIR / "images"

# ─── 工具函数 ────────────────────────────────────────────────────────────────

def strip_html_tags(text: str) -> str:
    """移除所有 HTML 标签，只保留纯文本内容。"""
    return re.sub(r'<[^>]+>', '', text)


def count_chinese_and_chars(text: str) -> int:
    """统计有效字符数（中文+英文+数字，去除空白）。"""
    clean = re.sub(r'\s+', '', text)
    return len(clean)


def clean_filename(text: str, max_len: int = 20) -> str:
    """
    将正文文字清理成合法的文件名。
    策略：只保留中文字、英文字母、数字，其余全部丢弃。
    """
    # 去 HTML 标签
    text = strip_html_tags(text)
    # 解码 HTML 实体
    text = html.unescape(text)
    # 去除零宽字符
    text = re.sub(r'[\u200b\u200c\u200d\ufeff]', '', text)
    # 去除换行、tab
    text = re.sub(r'[\r\n\t]', '', text)

    # 只保留：中文（包含CJK统一汉字）、英文字母、数字
    # 其余（标点、emoji、空格、引号等）全部去掉
    allowed = re.compile(r'[^\u4e00-\u9fff\u3400-\u4dbf\uff00-\uffef'
                         r'A-Za-z0-9]')
    text = allowed.sub('', text)

    # 截取前 max_len 个字符
    text = text[:max_len]
    return text


def decode_weibo_url(url: str) -> str:
    """
    处理微博外链包装：weibo.cn/sinaurl?u=https%3A%2F%2F...
    返回真实 URL。
    """
    if 'weibo.cn/sinaurl' in url or 'weibo.com/sinaurl' in url:
        parsed = urlparse(url)
        params = parse_qs(parsed.query)
        if 'u' in params:
            return unquote(params['u'][0])
    return url


def convert_text_to_markdown(raw_html: str) -> str:
    """
    将微博正文 HTML 转换为 Markdown 格式。
    处理：
    - <br> → 换行（两个空格+换行 => 段落内换行；连续<br><br>=> 空行）
    - 表情图片 <img alt="[xxx]" ...> → [xxx]
    - 图标链接图片（icon-link）→ 忽略 img，保留链接文字
    - <a href="...">文字</a> → [文字](url)（外链解码）
    - HTML 实体解码
    """
    text = raw_html

    # 1. 处理 <br> 标签
    # 连续两个 <br> → 空行（段落分隔）
    text = re.sub(r'(<br\s*/?>){2,}', '\n\n', text, flags=re.IGNORECASE)
    # 单个 <br> → 换行
    text = re.sub(r'<br\s*/?>', '\n', text, flags=re.IGNORECASE)

    # 2. 表情图片 <img alt="[xxx]" class="..." ...> → [xxx]
    def replace_emoji_img(m):
        alt = m.group(1)
        return alt if alt else ''
    text = re.sub(
        r'<img[^>]+alt="(\[[^\]]*\])"[^>]*>',
        replace_emoji_img,
        text
    )

    # 3. 处理链接 <a ...><img class="icon-link" ...>链接文字</a>
    #    这类链接里有个小图标，忽略图标，保留文字+url
    def replace_link_with_icon(m):
        href = m.group(1)
        inner = m.group(2)
        # 移除内部 icon-link img 标签
        inner_clean = re.sub(r'<img[^>]+class="icon-link"[^>]*>', '', inner)
        inner_clean = strip_html_tags(inner_clean).strip()
        href = decode_weibo_url(href)
        if inner_clean:
            return f'[{inner_clean}]({href})'
        else:
            return f'[链接]({href})'
    text = re.sub(
        r'<a[^>]+href="([^"]*)"[^>]*>(.*?)</a>',
        replace_link_with_icon,
        text,
        flags=re.DOTALL | re.IGNORECASE
    )

    # 4. 移除剩余 HTML 标签
    text = re.sub(r'<[^>]+>', '', text)

    # 5. HTML 实体解码（&amp; &lt; &gt; &#xxx; 等）
    text = html.unescape(text)

    # 6. 清理尾部 ​​​（微博末尾常见的不可见符号）
    text = text.replace('\u200b', '').replace('\u200e', '').replace('\u200f', '')
    text = re.sub(r'\s+$', '', text, flags=re.MULTILINE)

    # 7. 合并连续空行（最多保留两个换行）
    text = re.sub(r'\n{3,}', '\n\n', text)

    return text.strip()


def extract_images_from_post(post_html: str) -> list:
    """
    从单条微博 HTML 中提取图片列表（只取 media-large 中的，去重）。
    返回图片文件名列表（如 ['xxx.jpg', 'yyy.png']）。
    """
    # 提取 media-large div 的内容
    large_match = re.search(
        r'<div class="media media-large">(.*?)</div>\s*</div>\s*</div>',
        post_html,
        re.DOTALL
    )
    if not large_match:
        # 尝试更宽泛的匹配
        large_match = re.search(
            r'<div class="media media-large">(.*?)(?=<div class="media|</div></div>)',
            post_html,
            re.DOTALL
        )

    srcs = []
    seen = set()

    # 从 media-large 里提取
    if large_match:
        large_html = large_match.group(1)
        imgs = re.findall(r'<img class="image-new" src="([^"]+)"', large_html)
        for src in imgs:
            fname = Path(src).name
            if fname not in seen:
                seen.add(fname)
                srcs.append(fname)
    else:
        # 如果没有 media-large，从 media-small 里取（降级处理）
        small_match = re.search(
            r'<div class="media media-small">(.*?)(?=<div class="media|$)',
            post_html,
            re.DOTALL
        )
        if small_match:
            imgs = re.findall(r'<img class="image-new" src="([^"]+)"', small_match.group(1))
            for src in imgs:
                fname = Path(src).name
                if fname not in seen:
                    seen.add(fname)
                    srcs.append(fname)

    return srcs


def parse_weibo_posts(html_content: str) -> list:
    """
    解析 HTML 中所有微博条目。
    返回 list of dict: {date, text_html, images}
    """
    posts = []

    # 用正则切割出每个 speechless-post 块
    # 每个 post 以 <div class="speechless-post"> 开始，到下一个开始或结束
    post_pattern = re.compile(
        r'<div class="speechless-post">\s*(.*?)(?=<div class="speechless-post">|$)',
        re.DOTALL
    )

    for m in post_pattern.finditer(html_content):
        post_html = m.group(1)

        # 提取日期
        date_match = re.search(r'<span class="date">([^<]+)</span>', post_html)
        if not date_match:
            continue
        date_str = date_match.group(1).strip()
        # 格式：2026/03/21 19:14 → 2026-03-21 19:14
        date_str = date_str.replace('/', '-')

        # 提取正文（第一个 class="text" div）
        text_match = re.search(r'<div class="text">(.*?)</div>', post_html, re.DOTALL)
        if not text_match:
            continue
        text_html = text_match.group(1).strip()

        # 计算纯文字字数（去标签后）
        plain_text = strip_html_tags(text_html)
        plain_text = html.unescape(plain_text)
        plain_text = re.sub(r'\s+', '', plain_text)
        # 去掉末尾的 ​​​ 之类不可见字符
        plain_text = plain_text.replace('\u200b', '').replace('\u200e', '').replace('\u200f', '')
        # 去掉方括号表情（如[蜡烛]），保留其他文字来计数
        char_count = len(plain_text)

        if char_count < 10:
            continue

        # 提取图片
        images = extract_images_from_post(post_html)

        posts.append({
            'date': date_str,
            'text_html': text_html,
            'images': images,
        })

    return posts


def make_unique_path(path: Path) -> Path:
    """如果路径已存在，加序号后缀避免冲突。"""
    if not path.exists():
        return path
    stem = path.stem
    suffix = path.suffix
    parent = path.parent
    i = 2
    while True:
        new_path = parent / f"{stem}-{i}{suffix}"
        if not new_path.exists():
            return new_path
        i += 1


def generate_markdown(post: dict, images_rel_prefix: str) -> str:
    """
    生成单条微博的 Markdown 内容。
    images_rel_prefix: 从该 md 文件到 images/ 的相对路径，如 '../../../images'
    """
    date = post['date']
    body = convert_text_to_markdown(post['text_html'])
    images = post['images']

    # 确保每段之间都有空行：
    # convert_text_to_markdown 已经把连续<br><br>转为\n\n，单<br>转为\n
    # 这里统一把所有单\n也扩展为\n\n，让每行之间都空一行
    body = re.sub(r'\n{1,}', '\n\n', body)
    body = body.strip()

    lines = []
    lines.append('---')
    lines.append(f'date: "{date}"')
    lines.append('---')
    lines.append('')
    lines.append(body)

    if images:
        lines.append('')
        img_lines = []
        for img in images:
            img_path = f"{images_rel_prefix}/{img}"
            img_lines.append(f'![]({img_path})')
        lines.append('\n'.join(img_lines))

    return '\n'.join(lines)


def copy_image(src_name: str, copied_set: set) -> bool:
    """
    将图片从 FILES_DIR 复制到 IMAGES_DIR。
    返回是否成功复制（已复制过的跳过）。
    """
    if src_name in copied_set:
        return False
    src_path = FILES_DIR / src_name
    dst_path = IMAGES_DIR / src_name
    if src_path.exists():
        shutil.copy2(src_path, dst_path)
        copied_set.add(src_name)
        return True
    return False


def run():
    print("=" * 60)
    print("微博 HTML → Markdown 转换器")
    print("=" * 60)

    # 读取 HTML
    print(f"\n[1/4] 读取 HTML 文件: {HTML_FILE.name}")
    with open(HTML_FILE, 'r', encoding='utf-8', errors='replace') as f:
        html_content = f.read()
    print(f"      文件大小: {len(html_content) / 1024 / 1024:.1f} MB")

    # 创建输出目录
    OUTPUT_DIR.mkdir(exist_ok=True)
    IMAGES_DIR.mkdir(exist_ok=True)

    # 解析微博
    print("\n[2/4] 解析微博条目...")
    posts = parse_weibo_posts(html_content)
    print(f"      解析完成，共 {len(posts)} 条（已过滤字数 < 10 的条目）")

    # 生成 Markdown 文件 + 复制图片
    print("\n[3/4] 生成 Markdown 文件 + 复制图片...")
    copied_images = set()
    generated = 0
    skipped = 0
    image_not_found = []

    for i, post in enumerate(posts):
        try:
            date = post['date']   # e.g. "2026-03-21 19:14"
            date_parts = date.split(' ')[0].split('-')  # ['2026', '03', '21']
            if len(date_parts) != 3:
                skipped += 1
                continue
            year, month, day = date_parts[0], date_parts[1], date_parts[2]
            time_part = date.split(' ')[1] if ' ' in date else '00-00'

            # 清理文件名：取正文前 10 字
            plain = strip_html_tags(post['text_html'])
            plain = html.unescape(plain)
            fname_suffix = clean_filename(plain, max_len=20)
            if not fname_suffix:
                fname_suffix = time_part.replace(':', '-')

            filename = f"{year}-{month}-{day}-{fname_suffix}.md"

            # 创建目录
            post_dir = OUTPUT_DIR / year / month
            post_dir.mkdir(parents=True, exist_ok=True)

            # 相对路径：weibo/YYYY/MM/ → weibo/images/  需要 ../../images
            images_rel = '../../images'

            # 生成 Markdown
            md_content = generate_markdown(post, images_rel)

            # 写入文件
            out_path = make_unique_path(post_dir / filename)
            out_path.write_text(md_content, encoding='utf-8')
            generated += 1

            # 复制图片
            for img in post['images']:
                ok = copy_image(img, copied_images)
                if not ok and img not in copied_images:
                    image_not_found.append(img)

        except Exception as e:
            print(f"      [警告] 处理第 {i+1} 条时出错: {e}")
            skipped += 1
            continue

        if (i + 1) % 500 == 0:
            print(f"      进度: {i+1}/{len(posts)} ({(i+1)/len(posts)*100:.0f}%)")

    print(f"\n[4/4] 完成！")
    print(f"      生成 Markdown 文件: {generated} 个")
    print(f"      跳过（出错）: {skipped} 个")
    print(f"      复制图片: {len(copied_images)} 张")
    if image_not_found:
        print(f"      找不到的图片: {len(set(image_not_found))} 张")
    print(f"\n输出目录: {OUTPUT_DIR}")
    print(f"图片目录: {IMAGES_DIR}")
    print("=" * 60)


if __name__ == '__main__':
    run()
