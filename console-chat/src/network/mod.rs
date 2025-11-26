use super::action::Action;
use color_eyre::Result;

pub fn handle_network(action: Action) -> Result<Option<Action>> {
    Ok(match action {
        Action::PerformJoin(_) => Some(Action::OpenChat),
        Action::PerformLogin(_, _) => Some(Action::OpenHome),
        _ => None,
    })
}
