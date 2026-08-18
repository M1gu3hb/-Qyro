import Flutter
import UIKit
import XCTest

@_silgen_name("qyro_protocol_version_ptr")
private func qyroProtocolVersionPointer() -> UnsafePointer<UInt8>

@_silgen_name("qyro_protocol_version_len")
private func qyroProtocolVersionLength() -> UInt

class RunnerTests: XCTestCase {

  func testNativeProtocolVersion() {
    let length = Int(qyroProtocolVersionLength())
    XCTAssertGreaterThan(length, 0)
    XCTAssertLessThanOrEqual(length, 64)

    let bytes = UnsafeBufferPointer(
      start: qyroProtocolVersionPointer(),
      count: length
    )
    XCTAssertEqual(String(decoding: bytes, as: UTF8.self), "QYRO/1")
  }

}
