// Code generated by the multiversx-sc build system. DO NOT EDIT.

////////////////////////////////////////////////////
////////////////// AUTO-GENERATED //////////////////
////////////////////////////////////////////////////

// Init:                                 1
// Endpoints:                           20
// Async Callback (empty):               1
// Total number of exported functions:  22

#![no_std]

multiversx_sc_wasm_adapter::allocator!();
multiversx_sc_wasm_adapter::panic_handler!();

multiversx_sc_wasm_adapter::endpoints! {
    metabonding
    (
        init => init
        changeSigner => change_signer
        pause => pause_endpoint
        unpause => unpause_endpoint
        isPaused => paused_status
        addProject => add_project
        removeProject => remove_project
        clearExpiredProjects => clear_expired_projects
        getAllProjectIds => get_all_project_ids_view
        getProjectById => get_project_by_id
        getCurrentWeek => get_current_week
        addRewardsCheckpoint => add_rewards_checkpoint
        depositRewards => deposit_rewards
        getRewardsForWeek => get_rewards_for_week_pretty
        claimRewards => claim_rewards
        claimPartialRewards => claim_partial_rewards
        getUserClaimableWeeks => get_user_claimable_weeks
        clearOldStorageFlags => clear_old_storage_flags
        addSCAddressToWhitelist => add_sc_address_to_whitelist
        removeSCAddressFromWhitelist => remove_sc_address_from_whitelist
        isSCAddressWhitelisted => is_sc_address_whitelisted
    )
}

multiversx_sc_wasm_adapter::async_callback_empty! {}
