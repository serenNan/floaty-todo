"""Generate Floaty Todo source icon (1024x1024 PNG).
Black-and-white notebook with 3 todo rows (2 checked, 1 unchecked).
Mirrors the user-provided design.
"""
from PIL import Image, ImageDraw

W = 1024
img = Image.new("RGBA", (W, W), (0, 0, 0, 0))
d = ImageDraw.Draw(img)

# Outer rounded white card (iOS-style app icon shape)
CARD_INSET = 30
CARD_RADIUS = 200
d.rounded_rectangle(
    [CARD_INSET, CARD_INSET, W - CARD_INSET, W - CARD_INSET],
    radius=CARD_RADIUS,
    fill=(255, 255, 255, 255),
)

BLACK = (10, 10, 10, 255)
STROKE = 26  # main line weight

# Notebook body — rounded rectangle, leaves room above for spiral rings
NB_LEFT = 260
NB_RIGHT = W - 260
NB_TOP = 295
NB_BOTTOM = W - 200
NB_RADIUS = 60
d.rounded_rectangle(
    [NB_LEFT, NB_TOP, NB_RIGHT, NB_BOTTOM],
    radius=NB_RADIUS,
    outline=BLACK,
    width=STROKE,
)

# 3 spiral rings on top — vertical bars piercing the notebook top edge
RING_Y_TOP = 220
RING_Y_BOTTOM = NB_TOP + 60
RING_W = 30  # bar width
RING_RADIUS = 16
ring_xs = [NB_LEFT + 130, (NB_LEFT + NB_RIGHT) // 2, NB_RIGHT - 130]
for cx in ring_xs:
    d.rounded_rectangle(
        [cx - RING_W // 2, RING_Y_TOP, cx + RING_W // 2, RING_Y_BOTTOM],
        radius=RING_RADIUS,
        fill=BLACK,
    )

# Three rows: checkbox + line
ROW_YS = [NB_TOP + 175, NB_TOP + 320, NB_TOP + 465]
CB_CX = NB_LEFT + 130
CB_R = 50
LINE_X1 = CB_CX + 95
LINE_X2 = NB_RIGHT - 90
LINE_THICK = 36
LINE_RADIUS = 18

for i, cy in enumerate(ROW_YS):
    is_checked = i < 2
    # Checkbox circle
    if is_checked:
        d.ellipse(
            [CB_CX - CB_R, cy - CB_R, CB_CX + CB_R, cy + CB_R],
            fill=BLACK,
        )
        # White check mark inside
        check_thick = 16
        # ✓ shape: short stroke down-right, long stroke up-right
        d.line(
            [(CB_CX - 22, cy + 0), (CB_CX - 4, cy + 18), (CB_CX + 26, cy - 18)],
            fill=(255, 255, 255, 255),
            width=check_thick,
            joint="curve",
        )
    else:
        d.ellipse(
            [CB_CX - CB_R, cy - CB_R, CB_CX + CB_R, cy + CB_R],
            outline=BLACK,
            width=STROKE,
        )

    # The horizontal line (text placeholder)
    d.rounded_rectangle(
        [LINE_X1, cy - LINE_THICK // 2, LINE_X2, cy + LINE_THICK // 2],
        radius=LINE_RADIUS,
        fill=BLACK,
    )

out = "D:/Projects/Floaty-todo/icon-source.png"
img.save(out, "PNG")
print(f"wrote {out} ({W}x{W})")
