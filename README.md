# Metabonding

...

## Introduction

The metabonding smart contract is meant to give additional incentives to users to stake their tokens, specifically EGLD and LKMEX (Locked MEX). Each user that has some tokens staked will receive rewards based on how much they contribute to the specific staking pool.

This can act as either a launchpad for new projects, or just as an additional reward pool for stakers.

## Adding projects

Projects can only be added by the contract owner. Each project has:
- a unique ID of maximum 10 characters (bytes)
- an owner - will receive any leftover tokens once the project is cleared
- a reward token
- a total reward token supply
- a start week
- a duration in weeks
- a percentage of how much of the rewards is distributed to LKMEX stakers (the rest will be distributed to EGLD stakers)

Projects can also be removed by the owner if deemed necessary. All leftover funds will be returned to the project owner in such a scenario.

A project is not considered "active" until all reward tokens have been deposited.

## Rewards distribution

Rewards are distributed on a weekly basis. For example, if a project has a 4 week duration, then 25% of the rewards will be distributed each week. From this 25%, a part will be distributed to EGLD stakers, and a part to LKMEX stakers. This depends on the percentage given at the project's initialization. 

The owner will add weekly checkpoints, which will describe the total staking pool for both EGLD and LKMEX.

Distribution is not done automatically. Each user will have to claim their own rewards. They can do so until the project is expired, which is currently set to one week after its end.

Since the metabonding SC does not have access to the staking pool's information, it will receive these informations from the users when they claim. These are checked against a signature provided by the owner (or another designated signer address). The current implementation works like this:

- owner checks the staking pools, and gets the total amounts, then creates the checkpoint for the current week with those values
- owner checks the specific values for users, then the signature is given by `sign_ed25519(week_number + user_address + user_egld_staked_amount + user_lkmex_staked_amount)`. This is signed using the `signer`'s secret key
- the user claims rewards, by giving the week number, user_egld_staked_amount, user_lkmex_staked_amount and the signature as arguments. 
- the metabonding SC verifies the signature, and gives the user their share of the rewards
- the SC marks the rewards as claimed for the given week for the current user

## Rewards formula

The weekly reward formula is as follows:

Note: Percentages are considered to be in range [0, 100] in this example.  

total_weekly_reward = project_reward_supply / project_duration_weeks 
weekly_reward_lkmex = total_weekly_reward * lkmex_reward_percentage / 100  
weekly_reward_egld = total_weekly_reward - weekly_reward_lkmex  

user_weekly_reward_for_lkmex = weekly_reward_lkmex * user_lkmex_staked / total_lkmex_staked  
user_weekly_reward_for_egld = weekly_reward_egld * user_egld_staked / total_egld_staked

user_weekly_reward = user_weekly_reward_for_lkmex + user_weekly_reward_for_egld
