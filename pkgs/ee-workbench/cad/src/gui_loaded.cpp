#include "ee/gui.hpp"

#include <Gui/Application.h>

namespace ee::gui {

bool active()
{
    return Gui::Application::Instance != nullptr;
}

void fit_view()
{
    if (Gui::Application::Instance != nullptr) {
        Gui::Application::Instance->sendMsgToActiveView("ViewFit");
    }
}

void reset_view()
{
    if (Gui::Application::Instance != nullptr) {
        Gui::Application::Instance->sendMsgToActiveView("ViewAxo");
        Gui::Application::Instance->sendMsgToActiveView("ViewFit");
    }
}

}  // namespace ee::gui
