#import "XCWNativeBridge.h"

#import "DFPrivateSimulatorDisplayBridge.h"
#import "XCWAccessibilityBridge.h"
#import "XCWSimctl.h"

#import <AppKit/AppKit.h>
#import <CoreFoundation/CoreFoundation.h>
#import <CoreVideo/CoreVideo.h>
#include <stdlib.h>
#include <string.h>

static NSString *XCWStringFromCString(const char *value) {
    if (value == NULL) {
        return @"";
    }
    return [NSString stringWithUTF8String:value] ?: @"";
}

static char *XCWCopyCString(NSString *string) {
    NSData *data = [[string ?: @"" dataUsingEncoding:NSUTF8StringEncoding] copy];
    char *buffer = calloc(data.length + 1, sizeof(char));
    if (buffer == NULL) {
        return NULL;
    }
    memcpy(buffer, data.bytes, data.length);
    buffer[data.length] = '\0';
    return buffer;
}

static void XCWSetErrorMessage(char **errorMessage, NSError *error) {
    if (errorMessage == NULL) {
        return;
    }
    *errorMessage = XCWCopyCString(error.localizedDescription ?: @"Unknown native error.");
}

static char *XCWJSONStringFromObject(id object, char **errorMessage) {
    NSError *jsonError = nil;
    NSData *data = [NSJSONSerialization dataWithJSONObject:object options:0 error:&jsonError];
    if (data == nil) {
        XCWSetErrorMessage(errorMessage, jsonError);
        return NULL;
    }

    NSString *string = [[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding] ?: @"{}";
    return XCWCopyCString(string);
}

@interface XCWNativeAccessibilityThreadRunner : NSObject
@end

@implementation XCWNativeAccessibilityThreadRunner

+ (void)run {
    @autoreleasepool {
        NSThread.currentThread.name = @"com.sickdeck.native-accessibility";
        NSRunLoop *runLoop = NSRunLoop.currentRunLoop;
        [runLoop addPort:NSMachPort.port forMode:NSDefaultRunLoopMode];
        while (!NSThread.currentThread.cancelled) {
            @autoreleasepool {
                [runLoop runMode:NSDefaultRunLoopMode beforeDate:NSDate.distantFuture];
            }
        }
    }
}

@end

@interface XCWNativeAccessibilitySnapshotRequest : NSObject

@property (nonatomic, copy) NSString *udid;
@property (nonatomic, assign) BOOL hasPoint;
@property (nonatomic, assign) double x;
@property (nonatomic, assign) double y;
@property (nonatomic, assign) NSUInteger maxDepth;
@property (nonatomic, assign) BOOL interactiveOnly;
@property (nonatomic, assign) char *result;
@property (nonatomic, assign) char *serializationError;
@property (nonatomic, strong) NSError *snapshotError;

- (void)performSnapshot;

@end

@implementation XCWNativeAccessibilitySnapshotRequest

- (void)performSnapshot {
    @autoreleasepool {
        NSError *error = nil;
        NSValue *pointValue = self.hasPoint ? [NSValue valueWithPoint:NSMakePoint(self.x, self.y)] : nil;
        NSDictionary *snapshot = [XCWAccessibilityBridge accessibilitySnapshotForSimulatorUDID:self.udid
                                                                                       atPoint:pointValue
                                                                                     maxDepth:self.maxDepth
                                                                               interactiveOnly:self.interactiveOnly
                                                                                         error:&error];
        if (snapshot == nil) {
            self.snapshotError = error;
            return;
        }
        self.result = XCWJSONStringFromObject(snapshot, &_serializationError);
    }
}

@end

static NSThread *XCWNativeAccessibilityThread(void) {
    static NSThread *thread = nil;
    static dispatch_once_t onceToken;
    dispatch_once(&onceToken, ^{
        thread = [[NSThread alloc] initWithTarget:XCWNativeAccessibilityThreadRunner.class
                                        selector:@selector(run)
                                          object:nil];
        thread.name = @"com.sickdeck.native-accessibility";
        [thread start];
    });
    return thread;
}

static xcw_native_owned_bytes XCWOwnedBytesFromData(NSData *data) {
    xcw_native_owned_bytes bytes = {0};
    if (data.length == 0) {
        return bytes;
    }

    bytes.data = malloc(data.length);
    if (bytes.data == NULL) {
        return (xcw_native_owned_bytes){0};
    }
    memcpy(bytes.data, data.bytes, data.length);
    bytes.length = data.length;
    return bytes;
}

static BOOL XCWPerformSimctlAction(char **errorMessage, BOOL (^action)(XCWSimctl *simctl, NSError **error)) {
    XCWSimctl *simctl = [[XCWSimctl alloc] init];
    NSError *error = nil;
    BOOL ok = action(simctl, &error);
    if (!ok) {
        XCWSetErrorMessage(errorMessage, error);
    }
    return ok;
}

static NSDictionary *XCWSimulatorRecordForUDID(const char *udid, char **errorMessage) {
    XCWSimctl *simctl = [[XCWSimctl alloc] init];
    NSError *error = nil;
    NSDictionary *simulator = [simctl simulatorWithUDID:XCWStringFromCString(udid) error:&error];
    if (simulator == nil) {
        XCWSetErrorMessage(errorMessage, error);
    }
    return simulator;
}

void xcw_native_initialize_app(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        [NSApp setActivationPolicy:NSApplicationActivationPolicyProhibited];
    }
}

void xcw_native_run_main_loop_slice(double duration_seconds) {
    @autoreleasepool {
        if (duration_seconds <= 0) {
            duration_seconds = 0.01;
        }
        NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:duration_seconds];
        [[NSRunLoop mainRunLoop] runUntilDate:deadline];
    }
}

char *xcw_native_list_simulators(char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSArray<NSDictionary *> *simulators = [simctl listSimulatorsWithError:&error];
        if (simulators == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }
        return XCWJSONStringFromObject(@{ @"simulators": simulators }, error_message);
    }
}

char *xcw_native_simulator_creation_options(char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSDictionary *options = [simctl simulatorCreationOptionsWithError:&error];
        if (options == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }
        return XCWJSONStringFromObject(options, error_message);
    }
}

char *xcw_native_create_simulator(const char *name,
                                  const char *device_type_identifier,
                                  const char *runtime_identifier,
                                  const char *paired_watch_name,
                                  const char *paired_watch_device_type_identifier,
                                  const char *paired_watch_runtime_identifier,
                                  char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSDictionary *result = [simctl createSimulatorWithName:XCWStringFromCString(name)
                                          deviceTypeIdentifier:XCWStringFromCString(device_type_identifier)
                                             runtimeIdentifier:runtime_identifier == NULL ? nil : XCWStringFromCString(runtime_identifier)
                                               pairedWatchName:paired_watch_name == NULL ? nil : XCWStringFromCString(paired_watch_name)
                               pairedWatchDeviceTypeIdentifier:paired_watch_device_type_identifier == NULL ? nil : XCWStringFromCString(paired_watch_device_type_identifier)
                                  pairedWatchRuntimeIdentifier:paired_watch_runtime_identifier == NULL ? nil : XCWStringFromCString(paired_watch_runtime_identifier)
                                                         error:&error];
        if (result == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }
        return XCWJSONStringFromObject(result, error_message);
    }
}

bool xcw_native_boot_simulator(const char *udid, char **error_message) {
    @autoreleasepool {
        return XCWPerformSimctlAction(error_message, ^BOOL(XCWSimctl *simctl, NSError **error) {
            return [simctl bootSimulatorWithUDID:XCWStringFromCString(udid) error:error];
        });
    }
}

bool xcw_native_shutdown_simulator(const char *udid, char **error_message) {
    @autoreleasepool {
        return XCWPerformSimctlAction(error_message, ^BOOL(XCWSimctl *simctl, NSError **error) {
            return [simctl shutdownSimulatorWithUDID:XCWStringFromCString(udid) error:error];
        });
    }
}

bool xcw_native_toggle_appearance(const char *udid, char **error_message) {
    @autoreleasepool {
        return XCWPerformSimctlAction(error_message, ^BOOL(XCWSimctl *simctl, NSError **error) {
            return [simctl toggleAppearanceForSimulatorUDID:XCWStringFromCString(udid) error:error];
        });
    }
}

bool xcw_native_open_url(const char *udid, const char *url, char **error_message) {
    @autoreleasepool {
        return XCWPerformSimctlAction(error_message, ^BOOL(XCWSimctl *simctl, NSError **error) {
            return [simctl openURL:XCWStringFromCString(url)
                     simulatorUDID:XCWStringFromCString(udid)
                             error:error];
        });
    }
}

bool xcw_native_launch_bundle(const char *udid, const char *bundle_id, char **error_message) {
    @autoreleasepool {
        return XCWPerformSimctlAction(error_message, ^BOOL(XCWSimctl *simctl, NSError **error) {
            return [simctl launchBundleID:XCWStringFromCString(bundle_id)
                            simulatorUDID:XCWStringFromCString(udid)
                                    error:error];
        });
    }
}

xcw_native_owned_bytes xcw_native_screenshot_png(const char *udid, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSData *png = [simctl screenshotPNGForSimulatorUDID:XCWStringFromCString(udid)
                                                      error:&error];
        if (png == nil) {
            XCWSetErrorMessage(error_message, error);
            return (xcw_native_owned_bytes){0};
        }
        return XCWOwnedBytesFromData(png);
    }
}

xcw_native_owned_bytes xcw_native_screen_recording_mp4(const char *udid, double duration_seconds, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSData *mp4 = [simctl screenRecordingMP4ForSimulatorUDID:XCWStringFromCString(udid)
                                                 durationSeconds:duration_seconds
                                                           error:&error];
        if (mp4 == nil) {
            XCWSetErrorMessage(error_message, error);
            return (xcw_native_owned_bytes){0};
        }
        return XCWOwnedBytesFromData(mp4);
    }
}

char *xcw_native_start_screen_recording(const char *udid, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSString *recordingID = [simctl startScreenRecordingForSimulatorUDID:XCWStringFromCString(udid)
                                                                       error:&error];
        if (recordingID == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }
        return XCWCopyCString(recordingID);
    }
}

xcw_native_owned_bytes xcw_native_stop_screen_recording(const char *recording_id, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSData *mp4 = [simctl stopScreenRecordingWithID:XCWStringFromCString(recording_id)
                                                  error:&error];
        if (mp4 == nil) {
            XCWSetErrorMessage(error_message, error);
            return (xcw_native_owned_bytes){0};
        }
        return XCWOwnedBytesFromData(mp4);
    }
}

char *xcw_native_recent_logs(const char *udid, double seconds, size_t limit, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSArray<NSDictionary *> *entries = [simctl recentLogEntriesForSimulatorUDID:XCWStringFromCString(udid)
                                                                            seconds:seconds
                                                                              limit:limit
                                                                              error:&error];
        if (entries == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }

        return XCWJSONStringFromObject(@{ @"entries": entries }, error_message);
    }
}

char *xcw_native_accessibility_snapshot(const char *udid, bool has_point, double x, double y, size_t max_depth, bool interactive_only, char **error_message) {
    @autoreleasepool {
        XCWNativeAccessibilitySnapshotRequest *request = [XCWNativeAccessibilitySnapshotRequest new];
        request.udid = XCWStringFromCString(udid);
        request.hasPoint = has_point;
        request.x = x;
        request.y = y;
        request.maxDepth = max_depth;
        request.interactiveOnly = interactive_only;

        NSThread *accessibilityThread = XCWNativeAccessibilityThread();
        if (NSThread.currentThread == accessibilityThread) {
            [request performSnapshot];
        } else {
            [request performSelector:@selector(performSnapshot)
                             onThread:accessibilityThread
                           withObject:nil
                        waitUntilDone:YES
                                modes:@[NSDefaultRunLoopMode]];
        }

        if (request.result != NULL) {
            return request.result;
        }
        if (request.serializationError != NULL) {
            if (error_message != NULL) {
                *error_message = request.serializationError;
            } else {
                free(request.serializationError);
            }
            return NULL;
        }
        XCWSetErrorMessage(error_message, request.snapshotError);
        return NULL;
    }
}

static BOOL XCWTouchPhaseFromString(NSString *phase, DFPrivateSimulatorTouchPhase *outPhase, NSError **error) {
    NSString *phaseValue = phase.lowercaseString;
    if ([phaseValue isEqualToString:@"began"]) {
        *outPhase = DFPrivateSimulatorTouchPhaseBegan;
        return YES;
    }
    if ([phaseValue isEqualToString:@"moved"]) {
        *outPhase = DFPrivateSimulatorTouchPhaseMoved;
        return YES;
    }
    if ([phaseValue isEqualToString:@"ended"]) {
        *outPhase = DFPrivateSimulatorTouchPhaseEnded;
        return YES;
    }
    if ([phaseValue isEqualToString:@"cancelled"]) {
        *outPhase = DFPrivateSimulatorTouchPhaseCancelled;
        return YES;
    }
    if (error != NULL) {
        *error = [NSError errorWithDomain:@"SickDeck.NativeBridge"
                                     code:1
                                 userInfo:@{ NSLocalizedDescriptionKey: [NSString stringWithFormat:@"Unsupported touch phase `%@`.", phase ?: @""] }];
    }
    return NO;
}

/// CoreSimulator publishes each device type's panel geometry in a profile beside
/// the bundle, in pixels. Touches are dispatched normalized against it, and a
/// bridge that never learns it keeps its 1x1 default and puts every tap in the
/// top-left corner, so a missing size is an error rather than a default.
static CGSize XCWDisplayPixelSizeForDeviceType(NSString *deviceTypeIdentifier, CGFloat *scale) {
    if (scale != NULL) {
        *scale = 0.0;
    }
    if (deviceTypeIdentifier.length == 0) {
        return CGSizeZero;
    }

    NSString *root = @"/Library/Developer/CoreSimulator/Profiles/DeviceTypes";
    NSArray<NSString *> *bundles = [NSFileManager.defaultManager contentsOfDirectoryAtPath:root
                                                                                     error:NULL];

    for (NSString *bundle in bundles) {
        NSString *path = [root stringByAppendingPathComponent:bundle];
        if (![[NSBundle bundleWithPath:path].bundleIdentifier isEqualToString:deviceTypeIdentifier]) {
            continue;
        }

        NSString *profilePath = [path stringByAppendingPathComponent:@"Contents/Resources/profile.plist"];
        NSDictionary *profile = [NSDictionary dictionaryWithContentsOfFile:profilePath];
        CGFloat width = [profile[@"mainScreenWidth"] doubleValue];
        CGFloat height = [profile[@"mainScreenHeight"] doubleValue];

        if (width > 0.0 && height > 0.0) {
            if (scale != NULL) {
                CGFloat mainScreenScale = [profile[@"mainScreenScale"] doubleValue];
                *scale = mainScreenScale > 0.0 ? mainScreenScale : 1.0;
            }
            return CGSizeMake(width, height);
        }
    }

    return CGSizeZero;
}

/// The accessibility tree reports frames in points, and touches are normalized,
/// so a caller that wants to press what it just read has to divide by this.
bool xcw_native_display_size(const char *udid, double *width_points, double *height_points,
                             double *scale, char **error_message) {
    @autoreleasepool {
        NSDictionary *simulator = XCWSimulatorRecordForUDID(udid, error_message);
        if (simulator == nil) {
            return false;
        }

        CGFloat pixelScale = 0.0;
        CGSize pixels = XCWDisplayPixelSizeForDeviceType(simulator[@"deviceTypeIdentifier"], &pixelScale);
        if (pixels.width <= 0.0 || pixels.height <= 0.0) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"XCWNativeBridge"
                                                                  code:1
                                                              userInfo:@{
                NSLocalizedDescriptionKey: @"Unable to read the simulator's display size.",
            }]);
            return false;
        }

        *width_points = pixels.width / pixelScale;
        *height_points = pixels.height / pixelScale;
        *scale = pixelScale;
        return true;
    }
}

static DFPrivateSimulatorDisplayBridge *XCWInputBridgeForUDID(const char *udid, char **errorMessage) {
    NSError *error = nil;
    DFPrivateSimulatorDisplayBridge *bridge = [[DFPrivateSimulatorDisplayBridge alloc] initWithUDID:XCWStringFromCString(udid)
                                                                                      attachDisplay:NO
                                                                                              error:&error];
    if (bridge == nil) {
        XCWSetErrorMessage(errorMessage, error);
        return nil;
    }

    NSDictionary *simulator = XCWSimulatorRecordForUDID(udid, NULL);
    CGSize displaySize = XCWDisplayPixelSizeForDeviceType(simulator[@"deviceTypeIdentifier"], NULL);
    if (displaySize.width <= 0.0 || displaySize.height <= 0.0) {
        XCWSetErrorMessage(errorMessage, [NSError errorWithDomain:@"XCWNativeBridge"
                                                             code:1
                                                         userInfo:@{
            NSLocalizedDescriptionKey: @"Unable to read the simulator's display size; every touch would miss.",
        }]);
        [bridge disconnect];
        return nil;
    }

    [bridge updateInputDisplaySize:displaySize];
    return bridge;
}

bool xcw_native_send_touch(const char *udid, double x, double y, const char *phase, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *phaseError = nil;
        DFPrivateSimulatorTouchPhase touchPhase = DFPrivateSimulatorTouchPhaseMoved;
        if (!XCWTouchPhaseFromString(XCWStringFromCString(phase), &touchPhase, &phaseError)) {
            XCWSetErrorMessage(error_message, phaseError);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge sendTouchAtNormalizedX:x normalizedY:y phase:touchPhase error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

void *xcw_native_input_create(const char *udid, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return NULL;
        }
        return (__bridge_retained void *)bridge;
    }
}

void xcw_native_input_destroy(void *handle) {
    @autoreleasepool {
        if (handle == NULL) {
            return;
        }
        DFPrivateSimulatorDisplayBridge *bridge = CFBridgingRelease(handle);
        [bridge disconnect];
    }
}

bool xcw_native_input_display_size(void *handle, double *width, double *height) {
    @autoreleasepool {
        if (handle == NULL) {
            return false;
        }
        CGSize size = [(__bridge DFPrivateSimulatorDisplayBridge *)handle displaySize];
        if (width != NULL) {
            *width = size.width;
        }
        if (height != NULL) {
            *height = size.height;
        }
        return size.width > 0.0 && size.height > 0.0;
    }
}

bool xcw_native_input_send_touch(void *handle, double x, double y, const char *phase, char **error_message) {
    @autoreleasepool {
        if (handle == NULL) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"SickDeck.NativeInput"
                                                                   code:1
                                                               userInfo:@{NSLocalizedDescriptionKey: @"Native input handle is null."}]);
            return false;
        }
        NSError *phaseError = nil;
        DFPrivateSimulatorTouchPhase touchPhase = DFPrivateSimulatorTouchPhaseMoved;
        if (!XCWTouchPhaseFromString(XCWStringFromCString(phase), &touchPhase, &phaseError)) {
            XCWSetErrorMessage(error_message, phaseError);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [(__bridge DFPrivateSimulatorDisplayBridge *)handle sendTouchAtNormalizedX:x
                                                                                normalizedY:y
                                                                                      phase:touchPhase
                                                                                      error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_input_send_edge_touch(void *handle, double x, double y, const char *phase, uint32_t edge, char **error_message) {
    @autoreleasepool {
        if (handle == NULL) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"SickDeck.NativeInput"
                                                                   code:1
                                                               userInfo:@{NSLocalizedDescriptionKey: @"Native input handle is null."}]);
            return false;
        }
        NSError *phaseError = nil;
        DFPrivateSimulatorTouchPhase touchPhase = DFPrivateSimulatorTouchPhaseMoved;
        if (!XCWTouchPhaseFromString(XCWStringFromCString(phase), &touchPhase, &phaseError)) {
            XCWSetErrorMessage(error_message, phaseError);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [(__bridge DFPrivateSimulatorDisplayBridge *)handle sendEdgeTouchAtNormalizedX:x
                                                                                    normalizedY:y
                                                                                          phase:touchPhase
                                                                                           edge:(DFPrivateSimulatorTouchEdge)edge
                                                                                          error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_input_send_multitouch(void *handle, double x1, double y1, double x2, double y2, const char *phase, char **error_message) {
    @autoreleasepool {
        if (handle == NULL) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"SickDeck.NativeInput"
                                                                   code:1
                                                               userInfo:@{NSLocalizedDescriptionKey: @"Native input handle is null."}]);
            return false;
        }
        NSError *phaseError = nil;
        DFPrivateSimulatorTouchPhase touchPhase = DFPrivateSimulatorTouchPhaseMoved;
        if (!XCWTouchPhaseFromString(XCWStringFromCString(phase), &touchPhase, &phaseError)) {
            XCWSetErrorMessage(error_message, phaseError);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [(__bridge DFPrivateSimulatorDisplayBridge *)handle sendMultiTouchAtNormalizedX1:x1
                                                                                       normalizedY1:y1
                                                                                       normalizedX2:x2
                                                                                       normalizedY2:y2
                                                                                             phase:touchPhase
                                                                                             error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_input_send_key(void *handle, uint16_t key_code, uint32_t modifiers, char **error_message) {
    @autoreleasepool {
        if (handle == NULL) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"SickDeck.NativeInput"
                                                                   code:1
                                                               userInfo:@{NSLocalizedDescriptionKey: @"Native input handle is null."}]);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [(__bridge DFPrivateSimulatorDisplayBridge *)handle sendKeyCode:key_code
                                                                        modifiers:modifiers
                                                                            error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_input_send_key_event(void *handle, uint16_t key_code, bool down, char **error_message) {
    @autoreleasepool {
        if (handle == NULL) {
            XCWSetErrorMessage(error_message, [NSError errorWithDomain:@"SickDeck.NativeInput"
                                                                   code:1
                                                               userInfo:@{NSLocalizedDescriptionKey: @"Native input handle is null."}]);
            return false;
        }
        NSError *error = nil;
        BOOL ok = [(__bridge DFPrivateSimulatorDisplayBridge *)handle sendKeyCode:key_code
                                                                             down:down
                                                                            error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_send_key(const char *udid, uint16_t key_code, uint32_t modifiers, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge sendKeyCode:key_code modifiers:modifiers error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_send_key_event(const char *udid, uint16_t key_code, bool down, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge sendKeyCode:key_code down:down error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_press_home(const char *udid, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge pressHomeButton:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_open_app_switcher(const char *udid, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge openAppSwitcher:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_press_button(const char *udid, const char *button_name, uint32_t duration_ms, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge pressHardwareButtonNamed:XCWStringFromCString(button_name)
                                        durationMs:(NSUInteger)duration_ms
                                             error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_send_button(const char *udid, const char *button_name, bool pressed, bool has_usage, uint32_t usage_page, uint32_t usage, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge sendHardwareButtonNamed:XCWStringFromCString(button_name)
                                          pressed:pressed
                                        usagePage:has_usage ? @(usage_page) : nil
                                            usage:has_usage ? @(usage) : nil
                                            error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_rotate_crown(const char *udid, double delta, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge rotateDigitalCrownByDelta:delta error:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_rotate_right(const char *udid, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge rotateRight:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_rotate_left(const char *udid, char **error_message) {
    @autoreleasepool {
        DFPrivateSimulatorDisplayBridge *bridge = XCWInputBridgeForUDID(udid, error_message);
        if (bridge == nil) {
            return false;
        }
        NSError *error = nil;
        BOOL ok = [bridge rotateLeft:&error];
        [bridge disconnect];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_erase_simulator(const char *udid, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        BOOL ok = [simctl eraseSimulatorWithUDID:XCWStringFromCString(udid) error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_install_app(const char *udid, const char *app_path, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        BOOL ok = [simctl installAppAtPath:XCWStringFromCString(app_path)
                             simulatorUDID:XCWStringFromCString(udid)
                                      error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_uninstall_app(const char *udid, const char *bundle_id, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        BOOL ok = [simctl uninstallBundleID:XCWStringFromCString(bundle_id)
                              simulatorUDID:XCWStringFromCString(udid)
                                       error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

bool xcw_native_set_pasteboard_text(const char *udid, const char *text, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        BOOL ok = [simctl setPasteboardText:XCWStringFromCString(text)
                              simulatorUDID:XCWStringFromCString(udid)
                                       error:&error];
        if (!ok) {
            XCWSetErrorMessage(error_message, error);
        }
        return ok;
    }
}

char *xcw_native_get_pasteboard_text(const char *udid, char **error_message) {
    @autoreleasepool {
        XCWSimctl *simctl = [[XCWSimctl alloc] init];
        NSError *error = nil;
        NSString *text = [simctl pasteboardTextForSimulatorUDID:XCWStringFromCString(udid) error:&error];
        if (text == nil) {
            XCWSetErrorMessage(error_message, error);
            return NULL;
        }
        return XCWCopyCString(text);
    }
}

void xcw_native_free_string(char *value) {
    if (value != NULL) {
        free(value);
    }
}

void xcw_native_free_bytes(xcw_native_owned_bytes bytes) {
    if (bytes.data != NULL) {
        free(bytes.data);
    }
}
