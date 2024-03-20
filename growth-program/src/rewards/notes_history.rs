use crate::project::ProjectId;

use super::week_timekeeping::Week;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const MAX_NOTE_LENGTH: usize = 10;
pub const MAX_NOTE_HISTORY: usize = 20;

pub type NoteIndex = usize;
pub type NoteData<M> = ManagedBuffer<M>;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct Note<M: ManagedTypeApi> {
    pub note_data: NoteData<M>,
    pub week: Week,
}

#[multiversx_sc::module]
pub trait NotesHistoryModule: crate::project::ProjectsModule + crate::events::EventsModule {
    #[view(getNotesHistory)]
    fn get_notes_history(
        &self,
        project_id: ProjectId,
        user: ManagedAddress,
    ) -> MultiValueEncoded<Note<Self::Api>> {
        self.require_valid_project_id(project_id);

        let mut notes = MultiValueEncoded::new();
        let user_id = self.user_ids().get_id(&user);
        if user_id == NULL_ID {
            return notes;
        }

        let first_note_index = self.first_note_index(project_id, user_id).get();
        let current_note_index = self.current_note_index(project_id, user_id).get();
        for i in (first_note_index..current_note_index).rev() {
            let note = self.note(project_id, user_id, i).get();
            notes.push(note);
        }

        notes
    }

    fn insert_note(&self, project_id: ProjectId, user_id: AddressId, note: &Note<Self::Api>) {
        require!(note.note_data.len() <= MAX_NOTE_LENGTH, "Length too long");

        let index_mapper = self.current_note_index(project_id, user_id);
        let current_index = index_mapper.get();
        self.note(project_id, user_id, current_index).set(note);
        index_mapper.set(current_index + 1);

        if current_index < MAX_NOTE_HISTORY {
            return;
        }

        self.first_note_index(project_id, user_id)
            .update(|first_note_index| {
                self.note(project_id, user_id, *first_note_index).clear();
                *first_note_index += 1;
            });
    }

    #[storage_mapper("currentNoteIndex")]
    fn current_note_index(
        &self,
        project_id: ProjectId,
        user_id: AddressId,
    ) -> SingleValueMapper<NoteIndex>;

    #[storage_mapper("firstNoteIndex")]
    fn first_note_index(
        &self,
        project_id: ProjectId,
        user_id: AddressId,
    ) -> SingleValueMapper<NoteIndex>;

    #[storage_mapper("note")]
    fn note(
        &self,
        project_id: ProjectId,
        user_id: AddressId,
        note_index: NoteIndex,
    ) -> SingleValueMapper<Note<Self::Api>>;

    #[storage_mapper("userIds")]
    fn user_ids(&self) -> AddressToIdMapper<Self::Api>;
}
