#import "XCWNativeBridge.h"

#import <CoreMedia/CoreMedia.h>
#import <VideoToolbox/VideoToolbox.h>
#import <stdatomic.h>
#import <unistd.h>

#import "DFPrivateSimulatorDisplayBridge.h"

static const uint8_t XCWStartCode[] = {0, 0, 0, 1};

static int32_t XCWEven(double value) {
    int32_t rounded = (int32_t)llround(value);
    return MAX(2, rounded - (rounded & 1));
}

@interface XCWFrameStream : NSObject <DFPrivateSimulatorDisplayBridgeDelegate>
@end

@implementation XCWFrameStream {
    int32_t _targetWidth;
    VTCompressionSessionRef _session;
    VTPixelTransferSessionRef _transfer;
    size_t _sourceWidth;
    size_t _sourceHeight;
    BOOL _needsKeyFrame;
    atomic_bool _done;
}

- (instancetype)initWithWidth:(uint32_t)width {
    self = [super init];
    if (self != nil) {
        _targetWidth = (int32_t)width;
        atomic_init(&_done, false);
    }
    return self;
}

- (BOOL)isDone {
    return atomic_load(&_done);
}

- (void)finish {
    atomic_store(&_done, true);
}

- (void)invalidate {
    if (_session != NULL) {
        VTCompressionSessionInvalidate(_session);
        CFRelease(_session);
        _session = NULL;
    }
    if (_transfer != NULL) {
        VTPixelTransferSessionInvalidate(_transfer);
        CFRelease(_transfer);
        _transfer = NULL;
    }
}

- (void)dealloc {
    [self invalidate];
}

static void XCWEncoded(void *refcon, void *frameRefcon, OSStatus status, VTEncodeInfoFlags flags, CMSampleBufferRef sample) {
    (void)frameRefcon;
    (void)flags;
    XCWFrameStream *stream = (__bridge XCWFrameStream *)refcon;
    if (status != noErr || sample == NULL || !CMSampleBufferDataIsReady(sample) || [stream isDone]) {
        return;
    }
    [stream emit:sample];
}

- (void)emit:(CMSampleBufferRef)sample {
    NSMutableData *out = [NSMutableData data];
    CMFormatDescriptionRef format = CMSampleBufferGetFormatDescription(sample);
    int lengthSize = 4;

    CFArrayRef attachments = CMSampleBufferGetSampleAttachmentsArray(sample, false);
    BOOL keyFrame = attachments == NULL || CFArrayGetCount(attachments) == 0 ||
        !CFDictionaryContainsKey(CFArrayGetValueAtIndex(attachments, 0), kCMSampleAttachmentKey_NotSync);

    size_t count = 0;
    CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, 0, NULL, NULL, &count, &lengthSize);
    for (size_t i = 0; keyFrame && i < count; i++) {
        const uint8_t *set = NULL;
        size_t size = 0;
        if (CMVideoFormatDescriptionGetH264ParameterSetAtIndex(format, i, &set, &size, NULL, NULL) == noErr) {
            [out appendBytes:XCWStartCode length:sizeof XCWStartCode];
            [out appendBytes:set length:size];
        }
    }

    CMBlockBufferRef block = CMSampleBufferGetDataBuffer(sample);
    size_t total = CMBlockBufferGetDataLength(block);
    NSMutableData *avcc = [NSMutableData dataWithLength:total];
    if (CMBlockBufferCopyDataBytes(block, 0, total, avcc.mutableBytes) != kCMBlockBufferNoErr) {
        return;
    }

    const uint8_t *bytes = avcc.bytes;
    for (size_t at = 0; at + (size_t)lengthSize <= total;) {
        size_t length = 0;
        for (int i = 0; i < lengthSize; i++) {
            length = (length << 8) | bytes[at + (size_t)i];
        }
        at += (size_t)lengthSize;
        if (at + length > total) {
            break;
        }
        [out appendBytes:XCWStartCode length:sizeof XCWStartCode];
        [out appendBytes:bytes + at length:length];
        at += length;
    }

    const uint8_t *left = out.bytes;
    for (size_t remaining = out.length; remaining > 0;) {
        ssize_t wrote = write(STDOUT_FILENO, left, remaining);
        if (wrote < 0 && errno == EINTR) {
            continue;
        }
        if (wrote <= 0) {
            [self finish];
            return;
        }
        left += wrote;
        remaining -= (size_t)wrote;
    }
}

- (BOOL)prepareForWidth:(size_t)width height:(size_t)height {
    if (_session != NULL && width == _sourceWidth && height == _sourceHeight) {
        return YES;
    }
    [self invalidate];
    _sourceWidth = width;
    _sourceHeight = height;

    int32_t encodedWidth = XCWEven(MIN((double)_targetWidth, (double)width));
    int32_t encodedHeight = XCWEven((double)encodedWidth * (double)height / (double)width);

    NSDictionary *specification = @{
        (__bridge NSString *)kVTVideoEncoderSpecification_EnableLowLatencyRateControl: @YES,
    };
    OSStatus status = VTCompressionSessionCreate(kCFAllocatorDefault, encodedWidth, encodedHeight, kCMVideoCodecType_H264,
                                                 (__bridge CFDictionaryRef)specification, NULL, NULL,
                                                 XCWEncoded, (__bridge void *)self, &_session);
    if (status != noErr) {
        status = VTCompressionSessionCreate(kCFAllocatorDefault, encodedWidth, encodedHeight, kCMVideoCodecType_H264,
                                            NULL, NULL, NULL, XCWEncoded, (__bridge void *)self, &_session);
    }
    if (status != noErr || VTPixelTransferSessionCreate(kCFAllocatorDefault, &_transfer) != noErr) {
        [self invalidate];
        return NO;
    }

    VTSessionSetProperty(_session, kVTCompressionPropertyKey_RealTime, kCFBooleanTrue);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanFalse);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_H264_ConstrainedBaseline_AutoLevel);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_ExpectedFrameRate, (__bridge CFNumberRef)@60);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_MaxFrameDelayCount, (__bridge CFNumberRef)@0);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_AverageBitRate, (__bridge CFNumberRef)@2000000);
    VTSessionSetProperty(_session, kVTCompressionPropertyKey_MaxKeyFrameInterval, (__bridge CFNumberRef)@600);
    VTSessionSetProperty(_transfer, kVTPixelTransferPropertyKey_ScalingMode, kVTScalingMode_Normal);
    VTCompressionSessionPrepareToEncodeFrames(_session);

    _needsKeyFrame = YES;
    return YES;
}

- (void)privateSimulatorDisplayBridge:(DFPrivateSimulatorDisplayBridge *)bridge didUpdateFrame:(CVPixelBufferRef)pixelBuffer {
    (void)bridge;
    @synchronized(self) {
        [self encode:pixelBuffer];
    }
}

- (void)encode:(CVPixelBufferRef)pixelBuffer {
    if ([self isDone]) {
        return;
    }
    if (![self prepareForWidth:CVPixelBufferGetWidth(pixelBuffer) height:CVPixelBufferGetHeight(pixelBuffer)]) {
        [self finish];
        return;
    }

    CVPixelBufferRef scaled = NULL;
    CVPixelBufferPoolRef pool = VTCompressionSessionGetPixelBufferPool(_session);
    if (pool == NULL || CVPixelBufferPoolCreatePixelBuffer(kCFAllocatorDefault, pool, &scaled) != kCVReturnSuccess) {
        return;
    }
    if (VTPixelTransferSessionTransferImage(_transfer, pixelBuffer, scaled) != noErr) {
        CVPixelBufferRelease(scaled);
        return;
    }

    NSDictionary *options = _needsKeyFrame ? @{(__bridge NSString *)kVTEncodeFrameOptionKey_ForceKeyFrame: @YES} : nil;
    _needsKeyFrame = NO;
    VTCompressionSessionEncodeFrame(_session, scaled, CMClockGetTime(CMClockGetHostTimeClock()), kCMTimeInvalid,
                                    (__bridge CFDictionaryRef)options, NULL, NULL);
    CVPixelBufferRelease(scaled);
}

- (void)privateSimulatorDisplayBridge:(DFPrivateSimulatorDisplayBridge *)bridge
                didChangeDisplayStatus:(NSString *)status
                               isReady:(BOOL)isReady {
    (void)bridge;
    (void)status;
    (void)isReady;
}

@end

bool xcw_native_stream_h264(const char *udid, uint32_t width, char **error_message) {
    @autoreleasepool {
        XCWFrameStream *stream = [[XCWFrameStream alloc] initWithWidth:width];
        NSError *error = nil;
        DFPrivateSimulatorDisplayBridge *bridge =
            [[DFPrivateSimulatorDisplayBridge alloc] initWithUDID:[NSString stringWithUTF8String:udid]
                                                    attachDisplay:YES
                                                            error:&error];
        if (bridge == nil) {
            if (error_message != NULL) {
                *error_message = strdup((error.localizedDescription ?: @"cannot attach to the simulator display").UTF8String);
            }
            return false;
        }

        bridge.delegate = stream;
        CVPixelBufferRef current = [bridge copyPixelBuffer];
        if (current != NULL) {
            [stream privateSimulatorDisplayBridge:bridge didUpdateFrame:current];
            CVPixelBufferRelease(current);
        }

        while (![stream isDone]) {
            @autoreleasepool {
                [[NSRunLoop mainRunLoop] runUntilDate:[NSDate dateWithTimeIntervalSinceNow:0.25]];
            }
        }

        bridge.delegate = nil;
        [bridge disconnect];
        [stream invalidate];
        return true;
    }
}
