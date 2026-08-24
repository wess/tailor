# Tailor: the canvas

The canvas is where a design is made, and it is the part of Tailor that behaves
most like Interface Builder: an artboard at a device size, a selection with
knobs on it, a grid to catch drags, and four ways to look at the same document.

Everything here is about *this* page. For what you can place, see
[components and slots](tailorcomponents.md); for what the design becomes, see
[what gets generated](tailorcodegen.md).

## The artboard

The canvas holds one artboard, at the size the document says, and it scrolls.
Device presets sit across the toolbar — desktop, laptop, tablet, phone, panel,
square — with a rotate button beside them, and the size is editable if none of
them is what you want.

There is no magnification. gpui 0.2.2 has no transform for an arbitrary element
tree, so a zoom would scale some pixels and not others — text laid out at one
size and drawn at another, borders that stop being hairlines. Presets, rotate
and scroll are what the framework can do honestly.

The grid behind the artboard is a canvas affordance, not part of the design: it
never appears in the export. ⌘' shows or hides it; its spacing is a setting.

## Four modes

⌘1 – ⌘4, or the segmented control on the toolbar.

| Mode | What you get |
| --- | --- |
| **Design** | Real components, drawn from the real theme. A click selects rather than activates, so clicking a button selects the button. |
| **Blueprint** | Outlines and names only. For when the content is dense enough to hide the structure. |
| **Split** | Design on the left, the generated Rust on the right, regenerated on every edit. |
| **Preview** | Components go live and the canvas stops intercepting. Type in the fields, open the menus. Nothing you do here changes the document. |

Split is the one to leave on while you are learning guise. Drag a node and watch
`.gap(px(12.))` appear.

## Selecting

Click selects. ⌘-click adds to the selection. Esc selects the parent, which is
the fastest way out of a deep tree — press it repeatedly to walk up.

⌘A selects every sibling of the current selection rather than every node in the
document, because "all" inside a container is almost always what you meant.

The Outline is the other way to select, and it is the better one when the canvas
is dense: rows are drag sources and drop targets, named slots appear as their
own rows, and a row's lock and eye toggles are there.

**Locked** nodes cannot be selected on the canvas — use the Outline. **Hidden**
nodes disappear from the canvas and leave a marker in the generated code rather
than silently vanishing from it: hiding is a designer's affordance, not a
runtime condition, and a node you have hidden is one you are not ready to ship.

## Resizing

A selected node wears an outline and eight knobs, drawn on its edges rather than
inside them, so they never cover the content.

- Drag a **corner** to resize both axes; drag an **edge** for one.
- Drag the **body** of an absolutely-placed node to move it.
- Arrow keys **nudge** by the nudge distance; ⇧-arrows nudge by the grid.

Both drags work in deltas from where the drag started, not from the last frame,
so a drag the pointer outran does not drift. Both are a single undo step no
matter how many frames they took, and a knob you press without dragging leaves
no undo step at all.

There is a floor: a node cannot be dragged below 8×8. Smaller than that and the
knobs overlap each other and the node can never be grabbed again.

### What resizing actually changes

This is the part worth understanding, because Tailor resizes two different
things depending on what you grabbed.

A component that carries **its own pixel size** — an image, a chart, a
sparkline — is resized through those props. Dragging its corner writes
`width` and `height` on the component, the way Interface Builder resizes a
view's frame.

Everything else is resized through **the box around it**: the node's style
gets a width and a height, and the component fills it. That is the honest
mapping, because that is what the generated Rust does.

The inspector's Size tab shows which one you are editing.

## Layout: two modes, per container

Every container chooses how it arranges its children:

- **Flow** — gpui's flexbox. Direction, gap, alignment, wrap. This maps
  one-to-one onto real guise code, and it is the default.
- **Free form** — children carry an x and a y. The container generates as
  `relative()` and its children as `absolute().left(..).top(..)`.

Both generate real code. Flow is what you want for anything that reflows; free
form is what you want for overlays, badges, and pinned decoration.

⇧⌘G flips the selected container. If you work mostly free form, **New frames are
free form** in Settings makes every frame you drop start that way.

Align and Distribute do arithmetic on x/y inside a free-form container. Inside a
flow container there is nothing to move, so they set the *container's* alignment
instead — which is what you meant by "align these left" when a flexbox owns the
positions.

## Snapping

Two independent things, because wanting one without the other is the normal
case:

- **Snap to grid** catches a drag on the spacing you chose.
- **Snap to objects** catches it on a sibling's edge or centre, and draws a
  guide where it caught.

Both are on the Arrange menu and in Settings → Canvas, with the grid spacing and
the nudge distance beside them. ⇧⌘' toggles grid snapping without opening
anything.

## Right-click

Right-clicking a component selects it first and opens the menu second, so you
can always see what the next item is about to act on. Acting on something you
have not visibly selected is how a builder deletes the wrong node.

The node menu carries Rename, Duplicate, Cut/Copy/Paste, the Embed and Unwrap
commands, Move up/down, Lock, Hide, Select parent, Delete — and **Extract to a
component…**, which lifts the selection into a new component document and leaves
a reference behind.

Right-clicking the canvas itself offers Paste, Select all, and a new screen or
component. [Every other panel has a menu too](tailor.md#right-click).

## Restructuring without dragging

Dragging is not always the fastest way to change a tree.

- **Embed in frame / stack / card / scroll area** (⇧⌘E for a frame) wraps the
  selection in a new container, in place.
- **Unwrap** (⇧⌘U) lifts a container's children into its parent and deletes it.
- **Move up / Move down** reorders within a parent.

Between them you can restructure a whole screen from the keyboard.

## The live window

⌘⇧L opens a second OS window showing the document at its real device size, with
no canvas chrome and every component interactive. It updates on the same edit
that updates the canvas — leave it on a second display and watch the app take
shape while you work. It is the closest a compiled language gets to a live
preview.

### Inspecting it

⌥⌘I opens guise's own [DevTools](devtools.md) along the bottom of that window:
the Elements tree, the box model, resolved styles with the source line each came
from, plus Layers, Timelines and the rest.

It lives there rather than on the canvas because the live window renders the
document *alone* — the tree you get is your interface and nothing else, where
the same inspector on the workbench would show your design nested inside
Tailor's own panels.

Right-clicking the design gives you **Inspect element**, which does what the
same item does in a browser: selects the deepest component under the pointer,
scrolls the tree to it, and draws a box around it in the window. It opens the
inspector first if it was closed — the tree is recorded by the frames that
follow, so the pick waits for one rather than answering from an empty tree.

The inspector takes its room from the *window*, not from the design: opening it
makes the window bigger and leaves the document at its device size. Squeezing
the design to make space would defeat the one thing this window is for.

It is closed until you ask for it — the recorder behind the Elements tree only
runs while an inspector is alive, so a closed one costs the document nothing per
frame. **General → Inspect the live window** makes it open that way every time.

## Motion

The inspector's **Motion** tab gives a node an entrance. Pick one and it plays
on the canvas, in the live window, and in the generated code — the same
`Motion` in all three, built from the same settings, because a preview of
something other than what ships is worse than no preview.

| Setting | What it does |
| --- | --- |
| **Entrance** | None, Fade, Slide up / down / left / right. `None` is the default and turning it off is the same gesture as choosing a different one. |
| **Easing** | Sixteen of guise's curves, from Linear through Overshoot, Elastic, Bounce and Spring. |
| **Duration**, **Delay** | Milliseconds. |
| **Distance** | How far a slide travels, in px. Hidden for a fade, which does not travel. |
| **Stagger children** | Milliseconds between children — see below. Only offered on a node that has some. |
| **Repeat** | Once, or Loop, with **Alternate** to run every other pass backwards. |

**Stagger moves the animation off the node and onto its children**, one delay
per index. A container with a 60ms stagger does not animate itself: its first
child plays at 0, the second at 60, the third at 120. That is what stagger
means everywhere else, and it keeps a single node from having two animations to
reason about. A child with its own entrance keeps it and drops out of the wave.

Editing any of it replays the animation on the canvas immediately, and **Play
again** at the bottom of the tab replays every entrance in the document — a
mounted animation has already run, and this is what makes it run again.

A loop is honest about what it costs: an endless animation asks the window for
a frame forever. Use it for a hint, not for a screen.

What generates is one `.animate(..)` on the node's own box — a `Motion` builder
in the plain flavour, a `motion!` block in the macros one. See [what gets
generated](tailorcodegen.md#animation).

## Panels

Five regions, laid out the way Interface Builder and Android Studio's layout
editor lay theirs out.

| Region | What it is |
| --- | --- |
| **Library** (left) | Every component you can place, searchable, grouped by category, plus this project's own components. |
| **Outline** | The node tree, with slots as their own rows. |
| **Canvas** (centre) | The artboard. |
| **Inspector** (right) | Attributes, Size, Style, Motion, Connections, Identity. |
| **Problems** (bottom) | What will not generate, and what probably was not meant. |

Every panel resizes from the divider beside it and folds away from the chevron
in its header, leaving a rail you click to bring it back. Sizes and open/closed
state persist, so the layout you left is the one you come back to. The
inspector's sections fold individually and stay folded across selections.

⌥⌘1 – ⌥⌘4 toggle the four panels from the keyboard.

## Shortcuts

| Command | Shortcut |
| --- | --- |
| Undo / redo | ⌘Z / ⇧⌘Z |
| Duplicate | ⌘D |
| Delete | ⌫ |
| Select parent | Esc |
| Rename | ↵ |
| Embed in frame | ⇧⌘E |
| Unwrap | ⇧⌘U |
| Flow / free form | ⇧⌘G |
| Design / Blueprint / Split / Preview | ⌘1 … ⌘4 |
| Live window | ⇧⌘L |
| Developer tools | ⌥⌘I |
| Open in Editor | ⌥⌘O |
| Library / Outline / Inspector / Problems | ⌥⌘1 … ⌥⌘4 |
| Show grid / snap to grid / layout bounds | ⌘' / ⇧⌘' / ⇧⌘B |
| Nudge / nudge by the grid | Arrows / ⇧Arrows |
| Save / Save as / Export | ⌘S / ⇧⌘S / ⌘E |

## Undo

Undo is whole-project snapshots rather than an inverse-operation log. A project
is a few hundred nodes of plain data behind an `Arc`, so a snapshot is a
refcount bump and every operation is correct by construction rather than correct
if someone wrote the inverse properly.

Typing into a field collapses into one undo step rather than one per keystroke,
and a drag is one step rather than one per frame.
