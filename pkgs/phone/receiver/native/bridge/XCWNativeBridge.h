#import <Foundation/Foundation.h>

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

NS_ASSUME_NONNULL_BEGIN

typedef struct xcw_native_owned_bytes {
    uint8_t * _Nullable data;
    size_t length;
} xcw_native_owned_bytes;

void xcw_native_initialize_app(void);
void xcw_native_run_main_loop_slice(double duration_seconds);

char * _Nullable xcw_native_list_simulators(char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_simulator_creation_options(char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_create_simulator(const char * _Nonnull name,
                                             const char * _Nonnull device_type_identifier,
                                             const char * _Nullable runtime_identifier,
                                             const char * _Nullable paired_watch_name,
                                             const char * _Nullable paired_watch_device_type_identifier,
                                             const char * _Nullable paired_watch_runtime_identifier,
                                             char * _Nullable * _Nullable error_message);
bool xcw_native_boot_simulator(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_shutdown_simulator(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_toggle_appearance(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_open_url(const char * _Nonnull udid, const char * _Nonnull url, char * _Nullable * _Nullable error_message);
bool xcw_native_launch_bundle(const char * _Nonnull udid, const char * _Nonnull bundle_id, char * _Nullable * _Nullable error_message);
xcw_native_owned_bytes xcw_native_screenshot_png(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
xcw_native_owned_bytes xcw_native_screen_recording_mp4(const char * _Nonnull udid, double duration_seconds, char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_start_screen_recording(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
xcw_native_owned_bytes xcw_native_stop_screen_recording(const char * _Nonnull recording_id, char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_recent_logs(const char * _Nonnull udid, double seconds, size_t limit, char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_accessibility_snapshot(const char * _Nonnull udid, bool has_point, double x, double y, size_t max_depth, bool interactive_only, char * _Nullable * _Nullable error_message);
bool xcw_native_display_size(const char * _Nonnull udid, double * _Nonnull width_points, double * _Nonnull height_points, double * _Nonnull scale, char * _Nullable * _Nullable error_message);
bool xcw_native_send_touch(const char * _Nonnull udid, double x, double y, const char * _Nonnull phase, char * _Nullable * _Nullable error_message);
bool xcw_native_send_key(const char * _Nonnull udid, uint16_t key_code, uint32_t modifiers, char * _Nullable * _Nullable error_message);
bool xcw_native_send_key_event(const char * _Nonnull udid, uint16_t key_code, bool down, char * _Nullable * _Nullable error_message);
bool xcw_native_press_home(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_open_app_switcher(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_press_button(const char * _Nonnull udid, const char * _Nonnull button_name, uint32_t duration_ms, char * _Nullable * _Nullable error_message);
bool xcw_native_send_button(const char * _Nonnull udid, const char * _Nonnull button_name, bool pressed, bool has_usage, uint32_t usage_page, uint32_t usage, char * _Nullable * _Nullable error_message);
bool xcw_native_rotate_crown(const char * _Nonnull udid, double delta, char * _Nullable * _Nullable error_message);
bool xcw_native_rotate_right(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_rotate_left(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_erase_simulator(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
bool xcw_native_install_app(const char * _Nonnull udid, const char * _Nonnull app_path, char * _Nullable * _Nullable error_message);
bool xcw_native_uninstall_app(const char * _Nonnull udid, const char * _Nonnull bundle_id, char * _Nullable * _Nullable error_message);
bool xcw_native_set_pasteboard_text(const char * _Nonnull udid, const char * _Nonnull text, char * _Nullable * _Nullable error_message);
char * _Nullable xcw_native_get_pasteboard_text(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);

void * _Nullable xcw_native_input_create(const char * _Nonnull udid, char * _Nullable * _Nullable error_message);
void xcw_native_input_destroy(void * _Nullable handle);
bool xcw_native_input_display_size(void * _Nonnull handle, double * _Nullable width, double * _Nullable height);
bool xcw_native_input_send_touch(void * _Nonnull handle, double x, double y, const char * _Nonnull phase, char * _Nullable * _Nullable error_message);
bool xcw_native_input_send_edge_touch(void * _Nonnull handle, double x, double y, const char * _Nonnull phase, uint32_t edge, char * _Nullable * _Nullable error_message);
bool xcw_native_input_send_multitouch(void * _Nonnull handle, double x1, double y1, double x2, double y2, const char * _Nonnull phase, char * _Nullable * _Nullable error_message);
bool xcw_native_input_send_key(void * _Nonnull handle, uint16_t key_code, uint32_t modifiers, char * _Nullable * _Nullable error_message);
bool xcw_native_input_send_key_event(void * _Nonnull handle, uint16_t key_code, bool down, char * _Nullable * _Nullable error_message);


void xcw_native_free_string(char * _Nullable value);
void xcw_native_free_bytes(xcw_native_owned_bytes bytes);

NS_ASSUME_NONNULL_END
