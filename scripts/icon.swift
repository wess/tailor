// Renders the Tailor app icon — an artboard with selection knobs on a dark
// squircle — to a 1024x1024 PNG. Pure CoreGraphics, no third-party deps, so it
// runs anywhere Swift does (local + CI macOS runners). scripts/icon.sh turns
// the PNG into the .iconset/.icns. Usage: swift scripts/icon.swift out.png
//
// The mark is the thing Tailor does: a frame with eight handles around it. It
// reads as a layout tool at 16px, where a wordmark would not.
import CoreGraphics
import Foundation
import ImageIO

let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "assets/icon.png"
let dim = 1024
let space = CGColorSpaceCreateDeviceRGB()

func color(_ r: Double, _ g: Double, _ b: Double, _ a: Double = 1) -> CGColor {
  CGColor(colorSpace: space, components: [r, g, b, a])!
}

guard
  let ctx = CGContext(
    data: nil, width: dim, height: dim, bitsPerComponent: 8, bytesPerRow: 0,
    space: space, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
  )
else { fatalError("could not create context") }

let full = CGFloat(dim)
ctx.clear(CGRect(x: 0, y: 0, width: full, height: full))

// Rounded-rect "squircle" body, inset so the system shadow has room. Corner
// radius follows Apple's ~0.224 ratio of the body size.
let margin: CGFloat = 88
let body = CGRect(x: margin, y: margin, width: full - margin * 2, height: full - margin * 2)
let radius = body.width * 0.2237
let squircle = CGPath(roundedRect: body, cornerWidth: radius, cornerHeight: radius, transform: nil)

// Body fill: a vertical slate gradient, a shade cooler than Sinclair's indigo so
// the two are told apart in a dock.
ctx.saveGState()
ctx.addPath(squircle)
ctx.clip()
let bodyGrad = CGGradient(
  colorsSpace: space,
  colors: [color(0.16, 0.17, 0.22), color(0.06, 0.06, 0.09)] as CFArray,
  locations: [0, 1]
)!
ctx.drawLinearGradient(
  bodyGrad, start: CGPoint(x: 0, y: full), end: CGPoint(x: 0, y: 0), options: []
)

// The canvas grid Tailor draws behind an artboard, faint enough to read as
// texture rather than as content.
ctx.setStrokeColor(color(1, 1, 1, 0.05))
ctx.setLineWidth(2)
let step: CGFloat = 64
var g = body.minX
while g <= body.maxX {
  ctx.move(to: CGPoint(x: g, y: body.minY))
  ctx.addLine(to: CGPoint(x: g, y: body.maxY))
  g += step
}
g = body.minY
while g <= body.maxY {
  ctx.move(to: CGPoint(x: body.minX, y: g))
  ctx.addLine(to: CGPoint(x: body.maxX, y: g))
  g += step
}
ctx.strokePath()

// Soft sheen near the top for a little depth.
let sheen = CGGradient(
  colorsSpace: space,
  colors: [color(1, 1, 1, 0.10), color(1, 1, 1, 0)] as CFArray,
  locations: [0, 1]
)!
ctx.drawRadialGradient(
  sheen,
  startCenter: CGPoint(x: full / 2, y: full * 0.86), startRadius: 0,
  endCenter: CGPoint(x: full / 2, y: full * 0.86), endRadius: full * 0.6,
  options: []
)
ctx.restoreGState()

// Hairline top highlight on the rim.
ctx.saveGState()
ctx.addPath(squircle)
ctx.setStrokeColor(color(1, 1, 1, 0.06))
ctx.setLineWidth(3)
ctx.strokePath()
ctx.restoreGState()

// The artboard: a rounded frame with two component bars inside it.
let board = CGRect(x: 306, y: 336, width: 412, height: 352)
let boardPath = CGPath(roundedRect: board, cornerWidth: 26, cornerHeight: 26, transform: nil)

ctx.saveGState()
ctx.addPath(boardPath)
ctx.setFillColor(color(1, 1, 1, 0.07))
ctx.fillPath()
ctx.restoreGState()

ctx.saveGState()
ctx.setShadow(offset: .zero, blur: 30, color: color(0.42, 0.65, 1.0, 0.45))
ctx.addPath(boardPath)
ctx.setStrokeColor(color(0.62, 0.78, 1.0, 1))
ctx.setLineWidth(20)
ctx.strokePath()
ctx.restoreGState()

// Two placed components, the way a stat row sits on a screen.
ctx.saveGState()
ctx.setFillColor(color(0.62, 0.78, 1.0, 0.55))
for bar in [
  CGRect(x: 366, y: 556, width: 292, height: 44),
  CGRect(x: 366, y: 452, width: 188, height: 44),
] {
  ctx.addPath(CGPath(roundedRect: bar, cornerWidth: 22, cornerHeight: 22, transform: nil))
}
ctx.fillPath()
ctx.restoreGState()

// The selection knobs — the corner handles you drag to resize. Amber, because
// the selection is the one thing on the canvas that is not the design.
let knobR: CGFloat = 33
let knobs = [
  CGPoint(x: board.minX, y: board.minY),
  CGPoint(x: board.midX, y: board.minY),
  CGPoint(x: board.maxX, y: board.minY),
  CGPoint(x: board.minX, y: board.midY),
  CGPoint(x: board.maxX, y: board.midY),
  CGPoint(x: board.minX, y: board.maxY),
  CGPoint(x: board.midX, y: board.maxY),
  CGPoint(x: board.maxX, y: board.maxY),
]

ctx.saveGState()
ctx.setShadow(offset: .zero, blur: 26, color: color(1.0, 0.58, 0.12, 0.6))
for knob in knobs {
  ctx.addEllipse(
    in: CGRect(x: knob.x - knobR, y: knob.y - knobR, width: knobR * 2, height: knobR * 2))
}
ctx.setFillColor(color(1.0, 0.62, 0.16, 1))
ctx.fillPath()
ctx.restoreGState()

// A dark rim on each knob so they stay separate from the frame they sit on.
ctx.saveGState()
for knob in knobs {
  ctx.addEllipse(
    in: CGRect(x: knob.x - knobR, y: knob.y - knobR, width: knobR * 2, height: knobR * 2))
}
ctx.setStrokeColor(color(0.06, 0.06, 0.09, 0.85))
ctx.setLineWidth(8)
ctx.strokePath()
ctx.restoreGState()

guard let image = ctx.makeImage() else { fatalError("could not render image") }
let url = URL(fileURLWithPath: outPath)
guard
  let dest = CGImageDestinationCreateWithURL(
    url as CFURL, "public.png" as CFString, 1, nil
  )
else { fatalError("could not create \(outPath)") }
CGImageDestinationAddImage(dest, image, nil)
guard CGImageDestinationFinalize(dest) else { fatalError("could not write \(outPath)") }
print("wrote \(outPath)")
