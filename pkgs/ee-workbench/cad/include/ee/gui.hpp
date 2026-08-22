#pragma once

namespace ee::gui {

/// True only in the module loaded by the FreeCAD GUI. The headless server links
/// a stub, so session code can ask without knowing which binary it is in.
bool active();

/// Frame the active 3D view on everything it shows. A no-op headless.
void fit_view();

/// Point a freshly created view at the model from a corner. Only ever called on
/// a view nobody has aimed yet, so it steals no camera from the user.
void reset_view();

}  // namespace ee::gui
