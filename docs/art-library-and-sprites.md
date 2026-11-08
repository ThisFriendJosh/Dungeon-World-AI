# Art & codex uploads and AI sprites

Art & codex combines the supplied collection with artwork saved in the active campaign, including workshop tokens. The gallery loads thumbnails for one page at a time. Originals and recipes remain local in that campaign's SQLite save; switching campaigns clears the previous campaign's previews, upload queue, and selected artwork.

## Uploads and organization

Open **Art & codex → Add images** and choose up to 20 files for review. PNG, JPEG, WebP, SVG, GIF, BMP and AVIF are accepted when the installed WebView can decode them. Animated images use a single frame. Source files are limited to 20 MB; the table copy is normalized to PNG at up to 4096 pixels per edge and 4 MB, with a 256-pixel thumbnail. Raster originals are not retained; keep your original files separately when full resolution matters. Supported SVGs retain an inert, self-contained SVG copy for export as well as the normalized PNG.

Choose several categories per image: Battle maps, GM references, Scenes, World atlas, Creatures, Table styling, Characters, Items, Token portraits, or Unsorted. **All** shows every category. Add up to 30 custom tags, separated by commas, with up to 48 characters each. Search matches titles, categories, custom tags, and notes. Click a tag or choose it in **Filter by tag**. Use **View / edit tags** to change saved metadata later.

**Table use** determines whether the gallery offers **Use background**, **Place token**, or reference viewing only. GM references always use reference viewing in this gallery. Artwork never supplies character statistics or changes a character sheet automatically. Save the scene after placing artwork to keep its position.

The SVG importer supports shapes, text, groups, gradients, clipping, masks, and local references. Simple presentation styles become attributes. Scripts, external resources, foreignObject, stylesheets, and unsupported active markup are rejected with an explanation. Export a complex SVG as PNG if it uses unsupported features. SVGs are never inserted as executable page HTML.

Image imports, metadata edits, and generated token saves are transactional and record campaign activity. A failed save keeps the reviewed upload for retry. The shared image allowance is 500 images / 512 MB per campaign. Existing table uploads appear under Unsorted; existing workshop tokens appear under Token portraits and remain usable.

## AI token workshop

Open the workshop, enter an asset name and a sprite description, then choose **Generate AI sprite**. The frame, color, and short label remain separate controls. Preview the composition, then **Save PNG + SVG**. Both formats and the sprite recipe are saved; the SVG contains embedded raster artwork. Open a saved token to export either format or reuse its recipe. Tokens also appear in the gallery, where categories and custom tags can be edited.

Generation has progress and a Stop button. Cancelling or a model error does not save an asset or replace the previous successful preview. A seed repeats the requested variation; leave it blank for a new seed. Frame changes use the existing sprite. Background removal is a simple optional white-edge cutout, not a guaranteed foreground mask.

Image-model weights are a one-time separate installation. Open **Setup-Local-Art.cmd** and keep its window open until it reports completion, then restart the application. See [local artwork runtime](../services/art/README.md) for configuration and model terms. After setup, generation uses local weights with networking disabled in the model loader. No cloud fallback is configured. Uploads and tag editing work without the model.
