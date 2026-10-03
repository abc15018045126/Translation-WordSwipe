import os
import math
from PIL import Image, ImageDraw, ImageFont

def create_svg():
    svg_content = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="100%" height="100%">
  <defs>
    <!-- Background Gradient -->
    <linearGradient id="bgGrad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#1E3A8A" />
      <stop offset="45%" stop-color="#2563EB" />
      <stop offset="100%" stop-color="#06B6D4" />
    </linearGradient>

    <!-- Swipe Arc Gradient -->
    <linearGradient id="swipeGrad" x1="10%" y1="90%" x2="90%" y2="10%">
      <stop offset="0%" stop-color="#38BDF8" />
      <stop offset="50%" stop-color="#818CF8" />
      <stop offset="100%" stop-color="#C084FC" />
    </linearGradient>

    <!-- Glass Fill -->
    <linearGradient id="glassA" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.22" />
      <stop offset="100%" stop-color="#ffffff" stop-opacity="0.08" />
    </linearGradient>

    <linearGradient id="glassB" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#ffffff" stop-opacity="0.28" />
      <stop offset="100%" stop-color="#ffffff" stop-opacity="0.12" />
    </linearGradient>

    <!-- Soft Glow Filter -->
    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
      <feGaussianBlur stdDeviation="6" result="blur" />
      <feComposite in="SourceGraphic" in2="blur" operator="over" />
    </filter>
  </defs>

  <!-- Base Squircle Container -->
  <rect x="28" y="28" width="456" height="456" rx="108" fill="url(#bgGrad)" />
  <rect x="28" y="28" width="456" height="456" rx="108" fill="none" stroke="rgba(255, 255, 255, 0.25)" stroke-width="3.5" />

  <!-- Dynamic WordSwipe Ribbon (Background Glow & Curve) -->
  <path d="M 370 115 C 440 210, 310 270, 130 330 C 85 345, 110 395, 160 385 C 285 360, 440 300, 420 160 Z"
        fill="url(#swipeGrad)" opacity="0.3" filter="url(#glow)" />
  
  <path d="M 390 120 C 430 230, 280 275, 125 350"
        fill="none" stroke="url(#swipeGrad)" stroke-width="14" stroke-linecap="round" />

  <!-- Card 1: Top-Left "G" (Global / Source) -->
  <g transform="translate(72, 80)">
    <rect x="0" y="0" width="168" height="168" rx="38" fill="url(#glassA)" stroke="rgba(255, 255, 255, 0.35)" stroke-width="2.5" />
    <!-- Modern 'G' Vector Glyph -->
    <path d="M42.578,36.718C50,36.718,56.543,37.744,62.207,39.795L62.207,54.59C56.803,51.465,50.195,49.902,42.383,49.902C35.84,49.902,30.509,52.026,26.392,56.274C22.274,60.522,20.215,66.195,20.215,73.291C20.215,80.485,22.062,86.051,25.757,89.99C29.451,93.929,34.44,95.898,40.723,95.898C44.499,95.898,47.493,95.361,49.707,94.287L49.707,80.615L35.693,80.615L35.693,68.017L65.479,68.017L65.479,103.223C58.643,107.161,50.146,109.131,39.99,109.131C28.727,109.131,19.849,106.014,13.354,99.78C6.86,93.546,3.613,84.961,3.613,74.023C3.613,63.021,7.161,54.053,14.258,47.119C21.354,40.185,30.794,36.718,42.578,36.718z"
          transform="translate(36.3, -16.7) scale(1.381)"
          fill="#FFFFFF" />
  </g>

  <!-- Card 2: Bottom-Right "文" (Language / Target) -->
  <g transform="translate(272, 264)">
    <rect x="0" y="0" width="168" height="168" rx="38" fill="url(#glassB)" stroke="rgba(255, 255, 255, 0.45)" stroke-width="2.5" />
    <!-- Modern '文' Character Vector Glyph -->
    <path d="M33.447,50.976C36.8,61.653,42.236,70.882,49.756,78.662C57.34,70.849,62.744,61.621,65.967,50.976L33.447,50.976z M54.297,22.46C56.738,28.938,58.594,34.049,59.863,37.792L97.07,37.792L97.07,50.976L82.568,50.976C77.653,65.755,70.573,78.238,61.328,88.427C71.354,95.361,83.724,100.65,98.438,104.296C93.034,110.872,89.453,115.657,87.695,118.652C72.819,113.671,60.189,107.096,49.805,98.925C39.355,106.803,26.53,113.574,11.328,119.238C7.813,113.574,4.557,108.854,1.563,105.078C16.178,100.52,28.418,94.889,38.281,88.183C28.906,77.734,22.266,65.331,18.359,50.976L2.637,50.976L2.637,37.792L43.457,37.792C42.448,34.472,41.081,30.371,39.355,25.488L54.297,22.46z"
          transform="translate(29.8, 7.1) scale(1.085)"
          fill="#FFFFFF" />
  </g>

  <!-- Swipe Gesture Pulse & Spark Dot -->
  <circle cx="125" cy="350" r="10" fill="#FFFFFF" filter="url(#glow)" />
  <circle cx="125" cy="350" r="6" fill="#38BDF8" />
  <polygon points="400,105 408,120 423,128 408,136 400,151 392,136 377,128 392,120" fill="#FDE047" opacity="0.9" />
</svg>'''
    return svg_content

def draw_rendered_icon(size=1024):
    scale = size / 512.0
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 1. Background Gradient Squircle
    grad = Image.new("RGBA", (size, size))
    for y in range(size):
        for x in range(size):
            t = (x + y) / (2.0 * size)
            if t < 0.5:
                sub_t = t * 2.0
                r = int(30 * (1 - sub_t) + 37 * sub_t)
                g = int(58 * (1 - sub_t) + 99 * sub_t)
                b = int(138 * (1 - sub_t) + 235 * sub_t)
            else:
                sub_t = (t - 0.5) * 2.0
                r = int(37 * (1 - sub_t) + 6 * sub_t)
                g = int(99 * (1 - sub_t) + 182 * sub_t)
                b = int(235 * (1 - sub_t) + 212 * sub_t)
            grad.putpixel((x, y), (r, g, b, 255))

    mask = Image.new("L", (size, size), 0)
    mask_draw = ImageDraw.Draw(mask)
    margin = int(28 * scale)
    radius = int(108 * scale)
    mask_draw.rounded_rectangle([margin, margin, size - margin, size - margin], radius=radius, fill=255)
    img.paste(grad, (0, 0), mask)

    draw = ImageDraw.Draw(img)
    draw.rounded_rectangle([margin, margin, size - margin, size - margin], radius=radius, outline=(255, 255, 255, 60), width=max(1, int(3.5 * scale)))

    # 2. Dynamic swipe swoosh curve
    for step in range(120):
        t = step / 120.0
        u = 1 - t
        px = u**3 * 390 + 3*u**2*t * 410 + 3*u*t**2 * 260 + t**3 * 125
        py = u**3 * 120 + 3*u**2*t * 240 + 3*u*t**2 * 275 + t**3 * 350
        
        cr = int(56 * (1 - t) + 192 * t)
        cg = int(189 * (1 - t) + 132 * t)
        cb = int(248 * (1 - t) + 252 * t)
        
        pt_x = int(px * scale)
        pt_y = int(py * scale)
        r_arc = max(2, int(7 * scale))
        draw.ellipse([pt_x - r_arc, pt_y - r_arc, pt_x + r_arc, pt_y + r_arc], fill=(cr, cg, cb, 230))

    # 3. Card 1: Top-Left "G"
    c1_x0, c1_y0 = int(72 * scale), int(80 * scale)
    c1_w, c1_h = int(168 * scale), int(168 * scale)
    c1_rad = int(38 * scale)
    draw.rounded_rectangle([c1_x0, c1_y0, c1_x0 + c1_w, c1_y0 + c1_h], radius=c1_rad, fill=(255, 255, 255, 45), outline=(255, 255, 255, 90), width=max(1, int(2.5 * scale)))

    font_g_size = int(96 * scale)
    font_g = None
    try:
        font_g = ImageFont.truetype("C:/Windows/Fonts/segoeui.ttf", font_g_size)
    except Exception:
        font_g = ImageFont.load_default()
    
    bbox_g = font_g.getbbox("G")
    gw = bbox_g[2] - bbox_g[0]
    gh = bbox_g[3] - bbox_g[1]
    gx = c1_x0 + (c1_w - gw) // 2 - bbox_g[0]
    gy = c1_y0 + (c1_h - gh) // 2 - bbox_g[1]
    draw.text((gx, gy), "G", font=font_g, fill=(255, 255, 255, 255))

    # 4. Card 2: Bottom-Right "文"
    c2_x0, c2_y0 = int(272 * scale), int(264 * scale)
    c2_w, c2_h = int(168 * scale), int(168 * scale)
    c2_rad = int(38 * scale)
    draw.rounded_rectangle([c2_x0, c2_y0, c2_x0 + c2_w, c2_y0 + c2_h], radius=c2_rad, fill=(255, 255, 255, 55), outline=(255, 255, 255, 110), width=max(1, int(2.5 * scale)))

    font_wen_size = int(92 * scale)
    font_wen = None
    try:
        font_wen = ImageFont.truetype("C:/Windows/Fonts/msyh.ttc", font_wen_size)
    except Exception:
        try:
            font_wen = ImageFont.truetype("C:/Windows/Fonts/simhei.ttf", font_wen_size)
        except Exception:
            font_wen = ImageFont.load_default()
            
    bbox_wen = font_wen.getbbox("文")
    ww = bbox_wen[2] - bbox_wen[0]
    wh = bbox_wen[3] - bbox_wen[1]
    wx = c2_x0 + (c2_w - ww) // 2 - bbox_wen[0]
    wy = c2_y0 + (c2_h - wh) // 2 - bbox_wen[1]
    draw.text((wx, wy), "文", font=font_wen, fill=(255, 255, 255, 255))

    # Spark and Dot
    dot_x, dot_y = int(125 * scale), int(350 * scale)
    dot_r = max(2, int(6 * scale))
    draw.ellipse([dot_x - dot_r, dot_y - dot_r, dot_x + dot_r, dot_y + dot_r], fill=(56, 189, 248, 255))
    draw.ellipse([dot_x - dot_r // 2, dot_y - dot_r // 2, dot_x + dot_r // 2, dot_y + dot_r // 2], fill=(255, 255, 255, 255))

    return img

def export_all():
    # 1. Write SVG
    content = create_svg()
    with open('wordswipe_logo.svg', 'w', encoding='utf-8') as f:
        f.write(content)
    print("Generated wordswipe_logo.svg")

    # 2. Master 512x512 image
    master = draw_rendered_icon(512)
    master.save("icon_256.png", "PNG")
    print("Generated icon_256.png")

    # 3. Multi-resolution Windows ICO
    ico_sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    ico_images = [master.resize(s, Image.Resampling.LANCZOS) for s in ico_sizes]
    master.save("Translation WordSwipe.ico", format="ICO", sizes=ico_sizes)
    print("Generated Translation WordSwipe.ico (multi-resolution)")

    # 4. MSIX Assets
    os.makedirs("msix/Assets", exist_ok=True)
    
    # Square 150x150
    sq150 = master.resize((150, 150), Image.Resampling.LANCZOS)
    sq150.save("msix/Assets/Square150x150Logo.png", "PNG")
    
    # Square 44x44
    sq44 = master.resize((44, 44), Image.Resampling.LANCZOS)
    sq44.save("msix/Assets/Square44x44Logo.png", "PNG")
    sq44.save("msix/Assets/Square44x44Logo.targetsize-44.png", "PNG")
    sq44.save("msix/Assets/Square44x44Logo.altform-unplated_targetsize-44.png", "PNG")

    # StoreLogo 50x50
    store50 = master.resize((50, 50), Image.Resampling.LANCZOS)
    store50.save("msix/Assets/StoreLogo.png", "PNG")

    # Wide 310x150 (horizontal banner on navy background)
    wide = Image.new("RGBA", (310, 150), (15, 23, 42, 255))
    icon_in_wide = master.resize((110, 110), Image.Resampling.LANCZOS)
    wide.paste(icon_in_wide, ((310 - 110) // 2, (150 - 110) // 2), icon_in_wide)
    wide.save("msix/Assets/Wide310x150Logo.png", "PNG")

    # Splash 620x300
    splash = Image.new("RGBA", (620, 300), (15, 23, 42, 255))
    icon_in_splash = master.resize((180, 180), Image.Resampling.LANCZOS)
    splash.paste(icon_in_splash, ((620 - 180) // 2, (300 - 180) // 2), icon_in_splash)
    splash.save("msix/Assets/SplashScreen.png", "PNG")

    print("Generated all MSIX store assets in msix/Assets/")

if __name__ == '__main__':
    export_all()

