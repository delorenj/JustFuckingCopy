import Foundation
import Vision

guard CommandLine.arguments.count > 1 else {
    fputs("Missing image path.\n", stderr)
    exit(1)
}

let imageURL = URL(fileURLWithPath: CommandLine.arguments[1])
let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = true
request.recognitionLanguages = ["en-US"]

let handler = VNImageRequestHandler(url: imageURL, options: [:])

do {
    try handler.perform([request])
    let lines = (request.results as? [VNRecognizedTextObservation])?
        .compactMap { $0.topCandidates(1).first?.string }
        .filter { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty } ?? []

    print(lines.joined(separator: "\n"))
} catch {
    fputs("Vision OCR failed: \(error.localizedDescription)\n", stderr)
    exit(2)
}

