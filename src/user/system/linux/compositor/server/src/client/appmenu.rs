//! `xaqu_appmenu_v1`: an application's own menus for its toplevel.

use compositor_appmenu::{Builder, Step, Tree, Wrong};
use compositor_protocol::appmenu::{
    XAQU_APPMENU_V1, xaqu_appmenu_manager_v1 as manager, xaqu_appmenu_v1 as menu,
};
use compositor_wire::{Arg, ArgType, ObjectId};

use super::{Client, Event, Fatal, Role};

impl Client {
    /// The manager's `get_menu` and the menu's steps; `false` for a role
    /// that is not this module's.
    pub(super) fn appmenu(
        &mut self,
        sender: ObjectId,
        role: Role,
        version: u32,
        opcode: u16,
        args: &[Arg<'_>],
    ) -> bool {
        match role {
            Role::AppMenuManager if opcode == manager::request::GET_MENU => {
                let (Some(id), Some(toplevel)) = (
                    args.first().and_then(Arg::as_object),
                    args.get(1).and_then(Arg::as_object),
                ) else {
                    return true;
                };
                if !self.toplevels.contains_key(&toplevel) {
                    self.fail(Fatal::WrongInterface {
                        object: toplevel,
                        wanted: "xdg_toplevel",
                    });
                    return true;
                }
                self.forget_menus_of(toplevel);
                if self.make(id, &XAQU_APPMENU_V1, version, Role::AppMenu) {
                    let _ = self.appmenus.insert(id, toplevel);
                    let _ = self.builders.insert(id, Builder::default());
                }
                true
            }
            Role::AppMenuManager => true,
            Role::AppMenu => {
                self.appmenu_step(sender, opcode, args);
                true
            }
            _ => false,
        }
    }

    /// Drop the menu objects of `toplevel`: it has gone, or a second
    /// `get_menu` replaces them.
    pub(super) fn forget_menus_of(&mut self, toplevel: ObjectId) {
        let gone: Vec<ObjectId> = self
            .appmenus
            .iter()
            .filter(|(_, owner)| **owner == toplevel)
            .map(|(object, _)| *object)
            .collect();
        for object in gone {
            let _ = self.appmenus.remove(&object);
            let _ = self.builders.remove(&object);
        }
    }

    fn appmenu_step(&mut self, sender: ObjectId, opcode: u16, args: &[Arg<'_>]) {
        let text = |at: usize| {
            args.get(at)
                .and_then(Arg::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let uint = |at: usize| args.get(at).and_then(Arg::as_uint).unwrap_or_default();
        let step = match opcode {
            menu::request::ADD_MENU => Step::Menu(text(0)),
            menu::request::ADD_ITEM => Step::Item {
                id: uint(0),
                label: text(1),
                keys: text(2),
                flags: uint(3),
            },
            menu::request::ADD_SEPARATOR => Step::Line,
            menu::request::COMMIT => Step::Commit,
            _ => return,
        };
        let (Some(&toplevel), Some(builder)) =
            (self.appmenus.get(&sender), self.builders.get_mut(&sender))
        else {
            return;
        };
        match builder.apply(&step) {
            Ok(None) => {}
            Ok(Some(tree)) => {
                if let Some(it) = self.toplevels.get_mut(&toplevel) {
                    it.menu = (!tree.is_empty()).then_some(tree);
                }
                self.events.push(Event::MenuChanged);
            }
            Err(wrong) => {
                let code = match wrong {
                    Wrong::BadTree => menu::error::BAD_TREE,
                    Wrong::TooMany => menu::error::TOO_MANY,
                    Wrong::BadLabel => menu::error::BAD_LABEL,
                };
                self.fail(Fatal::Interface {
                    object: sender,
                    code,
                    text: wrong.to_string(),
                });
            }
        }
    }

    /// The own menus of the toplevel on `surface`, if it set any.
    #[must_use]
    pub fn menu_of(&self, surface: ObjectId) -> Option<&Tree> {
        self.toplevels
            .values()
            .find(|it| it.surface == surface)?
            .menu
            .as_ref()
    }

    /// Sends `activated(id)` to the menu object of the toplevel on
    /// `surface`, when `id` is one of its enabled items. Gives whether it
    /// was sent.
    pub fn activate_menu(&mut self, surface: ObjectId, id: u32) -> bool {
        let Some((&toplevel, _)) = self.toplevels.iter().find(|(_, it)| {
            it.surface == surface && it.menu.as_ref().is_some_and(|tree| tree.has(id))
        }) else {
            return false;
        };
        let Some(&object) = self
            .appmenus
            .iter()
            .find(|(_, owner)| **owner == toplevel)
            .map(|(object, _)| object)
        else {
            return false;
        };
        self.out
            .write(
                object,
                menu::event::ACTIVATED,
                &[ArgType::Uint],
                &[Arg::Uint(id)],
            )
            .is_ok()
    }
}
