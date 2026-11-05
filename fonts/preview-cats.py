#!/usr/bin/env python3
"""Export every coat pose and a local, timed pixel/font comparison viewer.

Use the same vendored PYTHONPATH as build-runcat-font.py. No browser or image
library is required to generate the review. A --font enables real COLR playback.
"""

import argparse
import base64
import io
import json
from pathlib import Path
import shutil
import html

import png
import pet_cats
import cat_coats


def png_data(pixels):
    buffer = io.BytesIO()
    png.Writer(len(pixels[0]), len(pixels), alpha=True, greyscale=False).write(
        buffer, [[component for pixel in row for component in pixel] for row in pixels])
    return buffer.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--font", type=Path)
    parser.add_argument("--fur", choices=cat_coats.CHOICES, default="tabby")
    args = parser.parse_args()
    coat = cat_coats.get(args.fur)
    args.output.mkdir(parents=True, exist_ok=True)
    if args.font:
        shutil.copyfile(args.font, args.output / "cat.ttf")
    _, layout, _ = pet_cats.poses(args.fur)
    actions = {}
    artwork = pet_cats.pixel_frames(args.fur)
    for index, frame in enumerate(artwork):
        name = frame["action"]
        number = len(actions.setdefault(name, []))
        glyphs = {}
        for cells in (1, 2, 3, 4):
            glyphs[str(cells)] = "".join(chr(code) for code, width, _, pose in layout
                                        if width == cells and pose == index)
            glyphs[f"{cells}l"] = "".join(chr(code) for code, width, _, pose in layout
                                          if width == cells and pose == index + len(artwork))
        # Retain the full source cell in the exported PNG; review both the
        # original one-line bounds and any accidental pixels outside them.
        data = png_data(frame["pixels"])
        (args.output / f"{name}-{number:02}.png").write_bytes(data)
        actions[name].append({"image": base64.b64encode(data).decode(),
                              "glyphs": glyphs, "breath": frame["breath"]})
    durations = {name: [100] * len(frames) for name, frames in actions.items()}
    durations["head-bump"] = [85] * 20
    durations["wall-scratch"] = [1350/16] * 16
    durations["parkour"] = [int(row.split("\t")[1]) for row in
                             (Path(__file__).parent / "pet-cats/parkour.tsv").read_text().splitlines()
                             if row and not row.startswith("#")]
    data = json.dumps({"actions": actions, "durations": durations, "font": bool(args.font)})
    face = "@font-face{font-family:Cat;src:url(cat.ttf)}" if args.font else ""
    page = TEMPLATE.replace("__DATA__", data).replace("__FONT_FACE__", face).replace("__COAT__", html.escape(coat.title))
    (args.output / "index.html").write_text(page)
    (args.output / "frames.json").write_text(data)
    print(args.output / "index.html")


TEMPLATE = r'''<!doctype html><meta charset="utf-8"><title>RunCat __COAT__ review</title>
<style>
__FONT_FACE__
*{box-sizing:border-box}body{font:15px system-ui;background:#181825;color:#cdd6f4;margin:28px}
button,select,input{font:inherit;margin:5px;padding:5px}h1{font-size:24px}h2{font-size:19px}
.controls{position:sticky;top:0;background:#181825;padding:8px;z-index:2;border:1px solid #45475a}
.live{display:flex;align-items:center;gap:30px;min-height:150px}.glyph{font-family:Cat;white-space:pre;line-height:1;color:#cdd6f4;font-synthesis:none}
.atlas{display:grid;grid-template-columns:repeat(8,190px);gap:8px}.tile{border:1px solid #45475a;padding:8px;overflow:hidden}
canvas{image-rendering:pixelated}.tile .glyph{display:inline-block;margin:7px 9px 0 0}
.light{background:#f1eadf;color:#302d29}.light .glyph{color:#302d29}
.row{padding:8px;display:flex;gap:20px;align-items:center}.small{font-size:13px;color:#a6adc8}
</style>
<h1>__COAT__ · frame and font review</h1>
<p>All 148 poses, with exact mirrored artwork. Enlarged pixels expose the markings;
the 20px / 24px font samples show their actual one-row footprint.</p>
<div class="controls"><select id="action"></select><button id="play">Pause</button>
<button id="prev">← frame</button><button id="next">frame →</button>
<label>Speed <select id="speed"><option value="1">1×</option><option value=".25">¼×</option><option value="2">2×</option></select></label>
<label><input type="checkbox" id="left">Mirror</label>
<label><input type="checkbox" id="light">Light background</label>
<span id="label"></span><div class="live"><canvas id="live" width="300" height="170"></canvas>
<div>20px · 3 cells<div class="glyph" id="font3" style="font-size:20px"></div></div>
<div>20px · 4 cells<div class="glyph" id="font4" style="font-size:20px"></div></div>
<div>24px · 4 cells<div class="glyph" id="font24" style="font-size:24px"></div></div></div></div>
<p class="small">Parkour uses its production 2.4s phase table; bump 1.7s; scratch 1.35s.
Other actions use a representative 10fps (live gait timing depends on typing).
The spin texture is sampled on the rotating 3D body, not painted over each image.</p>
<main id="atlases"></main><script>
const data=__DATA__, action=document.querySelector('#action'), left=document.querySelector('#left');
if(!data.font){document.querySelectorAll('.live>div').forEach(el=>el.hidden=true);document.head.insertAdjacentHTML('beforeend','<style>.tile .glyph{display:none}</style>');}
let images={}, selected='run', frame=0, playing=true, last=performance.now(), elapsed=0;
function draw(canvas,im,mirror,breath=0){
 const c=canvas.getContext('2d');c.clearRect(0,0,canvas.width,canvas.height);c.imageSmoothingEnabled=false;
 c.save();if(mirror){c.translate(canvas.width,0);c.scale(-1,1)}
 c.drawImage(im,10,16,30,16,0,canvas.height-16*canvas.width/30*(1+.04*breath),canvas.width,16*canvas.width/30*(1+.04*breath));c.restore();
}
function render(){const f=data.actions[selected][frame];document.querySelector('#label').textContent=`${selected} · ${frame+1}/${data.actions[selected].length} · ${data.durations[selected][frame]}ms`;
 draw(document.querySelector('#live'),images[selected][frame],left.checked,f.breath);
 for(const [id,size] of [['font3',3],['font4',4],['font24',4]])document.querySelector('#'+id).textContent=f.glyphs[size+(left.checked?'l':'')];}
function atlas(){for(const section of document.querySelectorAll('section')){
 const name=section.dataset.action;section.querySelectorAll('.tile').forEach((tile,i)=>{
 const f=data.actions[name][i];draw(tile.querySelector('canvas'),images[name][i],left.checked,f.breath);
 tile.querySelectorAll('.glyph').forEach(el=>el.textContent=f.glyphs[el.dataset.size+(left.checked?'l':'')]);});}}
Promise.all(Object.entries(data.actions).map(async([name,frames])=>{
 action.add(new Option(name,name));images[name]=await Promise.all(frames.map(f=>new Promise(resolve=>{const im=new Image();im.onload=()=>resolve(im);im.src='data:image/png;base64,'+f.image})));
 const section=document.createElement('section');section.dataset.action=name;section.innerHTML=`<h2>${name} · ${frames.length} frames</h2><div class="atlas">`+frames.map((f,i)=>`<div class="tile"><div>${i.toString().padStart(2,'0')} · ${data.durations[name][i]}ms</div><canvas width="168" height="96"></canvas><br><span class="glyph" data-size="3" style="font-size:20px"></span><span class="glyph" data-size="4" style="font-size:24px"></span></div>`).join('')+'</div>';
 document.querySelector('#atlases').append(section);
})).then(()=>{for(const name of Object.keys(data.actions))document.querySelector('#atlases').append(document.querySelector(`section[data-action="${name}"]`));action.value=selected;atlas();render();window.reviewReady=true;});
action.onchange=()=>{selected=action.value;frame=0;elapsed=0;render()};
document.querySelector('#play').onclick=e=>{playing=!playing;e.target.textContent=playing?'Pause':'Play'};
function step(d){playing=false;document.querySelector('#play').textContent='Play';frame=(frame+d+data.actions[selected].length)%data.actions[selected].length;elapsed=0;render()}
document.querySelector('#prev').onclick=()=>step(-1);document.querySelector('#next').onclick=()=>step(1);
left.onchange=()=>{atlas();render()};document.querySelector('#light').onchange=e=>document.body.classList.toggle('light',e.target.checked);
function tick(now){if(window.reviewReady&&playing){elapsed+=(now-last)*Number(document.querySelector('#speed').value);while(elapsed>=data.durations[selected][frame]){elapsed-=data.durations[selected][frame];frame=(frame+1)%data.actions[selected].length}render()}last=now;requestAnimationFrame(tick)}requestAnimationFrame(tick);
</script>'''


if __name__ == "__main__":
    main()
