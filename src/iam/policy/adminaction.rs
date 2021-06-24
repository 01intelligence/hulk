use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use super::*;
use crate::bucket::policy::{condition, Valid};

// Admin policy action.
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub struct AdminAction<'a>(&'a str);

impl<'a> std::convert::From<Action<'a>> for AdminAction<'a> {
    fn from(a: Action<'a>) -> Self {
        AdminAction(a.0)
    }
}

impl<'a> std::convert::From<&'a Action<'a>> for AdminAction<'a> {
    fn from(a: &'a Action<'a>) -> Self {
        AdminAction(a.0)
    }
}

// HEAL_ADMIN_ACTION - allows heal command
pub const HEAL_ADMIN_ACTION: AdminAction = AdminAction("admin:Heal");

// Service Actions

// STORAGE_INFO_ADMIN_ACTION - allow listing server info
pub const STORAGE_INFO_ADMIN_ACTION: AdminAction = AdminAction("admin:StorageInfo");
// PROMETHEUS_ADMIN_ACTION - prometheus info action
pub const PROMETHEUS_ADMIN_ACTION: AdminAction = AdminAction("admin:Prometheus");
// DATA_USAGE_INFO_ADMIN_ACTION - allow listing data usage info
pub const DATA_USAGE_INFO_ADMIN_ACTION: AdminAction = AdminAction("admin:DataUsageInfo");
// FORCE_UNLOCK_ADMIN_ACTION - allow force unlocking locks
pub const FORCE_UNLOCK_ADMIN_ACTION: AdminAction = AdminAction("admin:ForceUnlock");
// TOP_LOCKS_ADMIN_ACTION - allow listing top locks
pub const TOP_LOCKS_ADMIN_ACTION: AdminAction = AdminAction("admin:TopLocksInfo");
// PROFILING_ADMIN_ACTION - allow profiling
pub const PROFILING_ADMIN_ACTION: AdminAction = AdminAction("admin:Profiling");
// TRACE_ADMIN_ACTION - allow listing server trace
pub const TRACE_ADMIN_ACTION: AdminAction = AdminAction("admin:ServerTrace");
// CONSOLE_LOG_ADMIN_ACTION - allow listing console logs on terminal
pub const CONSOLE_LOG_ADMIN_ACTION: AdminAction = AdminAction("admin:ConsoleLog");
// KMS_CREATE_KEY_ADMIN_ACTION - allow creating a new KMS master key
pub const KMS_CREATE_KEY_ADMIN_ACTION: AdminAction = AdminAction("admin:KMSCreateKey");
// KMS_KEY_STATUS_ADMIN_ACTION - allow getting KMS key status
pub const KMS_KEY_STATUS_ADMIN_ACTION: AdminAction = AdminAction("admin:KMSKeyStatus");
// SERVER_INFO_ADMIN_ACTION - allow listing server info
pub const SERVER_INFO_ADMIN_ACTION: AdminAction = AdminAction("admin:ServerInfo");
// HEALTH_INFO_ADMIN_ACTION - allow obtaining cluster health information
pub const HEALTH_INFO_ADMIN_ACTION: AdminAction = AdminAction("admin:OBDInfo");
// BANDWIDTH_MONITOR_ACTION - allow monitoring bandwidth usage
pub const BANDWIDTH_MONITOR_ACTION: AdminAction = AdminAction("admin:BandwidthMonitor");

// SERVER_UPDATE_ADMIN_ACTION - allow MinIO binary update
pub const SERVER_UPDATE_ADMIN_ACTION: AdminAction = AdminAction("admin:ServerUpdate");
// SERVICE_RESTART_ADMIN_ACTION - allow restart of MinIO service.
pub const SERVICE_RESTART_ADMIN_ACTION: AdminAction = AdminAction("admin:ServiceRestart");
// SERVICE_STOP_ADMIN_ACTION - allow stopping MinIO service.
pub const SERVICE_STOP_ADMIN_ACTION: AdminAction = AdminAction("admin:ServiceStop");

// CONFIG_UPDATE_ADMIN_ACTION - allow MinIO config management
pub const CONFIG_UPDATE_ADMIN_ACTION: AdminAction = AdminAction("admin:ConfigUpdate");

// CREATE_USER_ADMIN_ACTION - allow creating MinIO user
pub const CREATE_USER_ADMIN_ACTION: AdminAction = AdminAction("admin:CreateUser");
// DELETE_USER_ADMIN_ACTION - allow deleting MinIO user
pub const DELETE_USER_ADMIN_ACTION: AdminAction = AdminAction("admin:DeleteUser");
// LIST_USERS_ADMIN_ACTION - allow list users permission
pub const LIST_USERS_ADMIN_ACTION: AdminAction = AdminAction("admin:ListUsers");
// ENABLE_USER_ADMIN_ACTION - allow enable user permission
pub const ENABLE_USER_ADMIN_ACTION: AdminAction = AdminAction("admin:EnableUser");
// DISABLE_USER_ADMIN_ACTION - allow disable user permission
pub const DISABLE_USER_ADMIN_ACTION: AdminAction = AdminAction("admin:DisableUser");
// GET_USER_ADMIN_ACTION - allows GET permission on user info
pub const GET_USER_ADMIN_ACTION: AdminAction = AdminAction("admin:GetUser");

// Service account Actions

// CREATE_SERVICE_ACCOUNT_ADMIN_ACTION - allow create a service account for a user
pub const CREATE_SERVICE_ACCOUNT_ADMIN_ACTION: AdminAction =
    AdminAction("admin:CreateServiceAccount");
// UPDATE_SERVICE_ACCOUNT_ADMIN_ACTION - allow updating a service account
pub const UPDATE_SERVICE_ACCOUNT_ADMIN_ACTION: AdminAction =
    AdminAction("admin:UpdateServiceAccount");
// REMOVE_SERVICE_ACCOUNT_ADMIN_ACTION - allow removing a service account
pub const REMOVE_SERVICE_ACCOUNT_ADMIN_ACTION: AdminAction =
    AdminAction("admin:RemoveServiceAccount");
// LIST_SERVICE_ACCOUNTS_ADMIN_ACTION - allow listing service accounts
pub const LIST_SERVICE_ACCOUNTS_ADMIN_ACTION: AdminAction =
    AdminAction("admin:ListServiceAccounts");

// Group Actions

// ADD_USER_TO_GROUP_ADMIN_ACTION - allow adding user to group permission
pub const ADD_USER_TO_GROUP_ADMIN_ACTION: AdminAction = AdminAction("admin:AddUserToGroup");
// REMOVE_USER_FROM_GROUP_ADMIN_ACTION - allow removing user to group permission
pub const REMOVE_USER_FROM_GROUP_ADMIN_ACTION: AdminAction =
    AdminAction("admin:RemoveUserFromGroup");
// GET_GROUP_ADMIN_ACTION - allow getting group info
pub const GET_GROUP_ADMIN_ACTION: AdminAction = AdminAction("admin:GetGroup");
// LIST_GROUPS_ADMIN_ACTION - allow list groups permission
pub const LIST_GROUPS_ADMIN_ACTION: AdminAction = AdminAction("admin:ListGroups");
// ENABLE_GROUP_ADMIN_ACTION - allow enable group permission
pub const ENABLE_GROUP_ADMIN_ACTION: AdminAction = AdminAction("admin:EnableGroup");
// DISABLE_GROUP_ADMIN_ACTION - allow disable group permission
pub const DISABLE_GROUP_ADMIN_ACTION: AdminAction = AdminAction("admin:DisableGroup");

// Policy Actions

// CREATE_POLICY_ADMIN_ACTION - allow create policy permission
pub const CREATE_POLICY_ADMIN_ACTION: AdminAction = AdminAction("admin:CreatePolicy");
// DELETE_POLICY_ADMIN_ACTION - allow delete policy permission
pub const DELETE_POLICY_ADMIN_ACTION: AdminAction = AdminAction("admin:DeletePolicy");
// GET_POLICY_ADMIN_ACTION - allow get policy permission
pub const GET_POLICY_ADMIN_ACTION: AdminAction = AdminAction("admin:GetPolicy");
// ATTACH_POLICY_ADMIN_ACTION - allows attaching a policy to a user/group
pub const ATTACH_POLICY_ADMIN_ACTION: AdminAction = AdminAction("admin:AttachUserOrGroupPolicy");
// LIST_USER_POLICIES_ADMIN_ACTION - allows listing user policies
pub const LIST_USER_POLICIES_ADMIN_ACTION: AdminAction = AdminAction("admin:ListUserPolicies");

// Bucket quota Actions

// SET_BUCKET_QUOTA_ADMIN_ACTION - allow setting bucket quota
pub const SET_BUCKET_QUOTA_ADMIN_ACTION: AdminAction = AdminAction("admin:SetBucketQuota");
// GET_BUCKET_QUOTA_ADMIN_ACTION - allow getting bucket quota
pub const GET_BUCKET_QUOTA_ADMIN_ACTION: AdminAction = AdminAction("admin:GetBucketQuota");

// Bucket Target admin Actions

// SET_BUCKET_TARGET_ACTION - allow setting bucket target
pub const SET_BUCKET_TARGET_ACTION: AdminAction = AdminAction("admin:SetBucketTarget");
// GET_BUCKET_TARGET_ACTION - allow getting bucket targets
pub const GET_BUCKET_TARGET_ACTION: AdminAction = AdminAction("admin:GetBucketTarget");

// Remote Tier admin Actions

// SET_TIER_ACTION - allow adding/editing a remote tier
pub const SET_TIER_ACTION: AdminAction = AdminAction("admin:SetTier");
// LIST_TIER_ACTION - allow listing remote tiers
pub const LIST_TIER_ACTION: AdminAction = AdminAction("admin:ListTier");

// LIST_POOLS_ACTION - list pools action
pub const LIST_POOLS_ACTION: AdminAction = AdminAction("admin:ListPools");

// ALL_ADMIN_ACTIONS - provides all admin permissions
pub const ALL_ADMIN_ACTIONS: AdminAction = AdminAction("admin:*");

lazy_static! {
    static ref SUPPORTED_ADMIN_ACTIONS: HashSet<AdminAction<'static>> = maplit::hashset! {
        HEAL_ADMIN_ACTION,
        STORAGE_INFO_ADMIN_ACTION,
        DATA_USAGE_INFO_ADMIN_ACTION,
        TOP_LOCKS_ADMIN_ACTION,
        PROFILING_ADMIN_ACTION,
        PROMETHEUS_ADMIN_ACTION,
        TRACE_ADMIN_ACTION,
        CONSOLE_LOG_ADMIN_ACTION,
        KMS_KEY_STATUS_ADMIN_ACTION,
        SERVER_INFO_ADMIN_ACTION,
        HEALTH_INFO_ADMIN_ACTION,
        BANDWIDTH_MONITOR_ACTION,
        SERVER_UPDATE_ADMIN_ACTION,
        SERVICE_RESTART_ADMIN_ACTION,
        SERVICE_STOP_ADMIN_ACTION,
        CONFIG_UPDATE_ADMIN_ACTION,
        CREATE_USER_ADMIN_ACTION,
        DELETE_USER_ADMIN_ACTION,
        LIST_USERS_ADMIN_ACTION,
        ENABLE_USER_ADMIN_ACTION,
        DISABLE_USER_ADMIN_ACTION,
        GET_USER_ADMIN_ACTION,
        ADD_USER_TO_GROUP_ADMIN_ACTION,
        REMOVE_USER_FROM_GROUP_ADMIN_ACTION,
        GET_GROUP_ADMIN_ACTION,
        LIST_GROUPS_ADMIN_ACTION,
        ENABLE_GROUP_ADMIN_ACTION,
        DISABLE_GROUP_ADMIN_ACTION,
        CREATE_SERVICE_ACCOUNT_ADMIN_ACTION,
        UPDATE_SERVICE_ACCOUNT_ADMIN_ACTION,
        REMOVE_SERVICE_ACCOUNT_ADMIN_ACTION,
        LIST_SERVICE_ACCOUNTS_ADMIN_ACTION,
        CREATE_POLICY_ADMIN_ACTION,
        DELETE_POLICY_ADMIN_ACTION,
        GET_POLICY_ADMIN_ACTION,
        ATTACH_POLICY_ADMIN_ACTION,
        LIST_USER_POLICIES_ADMIN_ACTION,
        SET_BUCKET_QUOTA_ADMIN_ACTION,
        GET_BUCKET_QUOTA_ADMIN_ACTION,
        SET_BUCKET_TARGET_ACTION,
        GET_BUCKET_TARGET_ACTION,
        SET_TIER_ACTION,
        LIST_TIER_ACTION,
        LIST_POOLS_ACTION,
        ALL_ADMIN_ACTIONS,
    };
    pub(super) static ref ADMIN_ACTION_CONDITION_KEY_MAP: HashMap<AdminAction<'static>, condition::KeySet<'static>> = {
        use crate::bucket::policy::condition::*;
        let all_admin_actions: KeySet<'static> = condition::ALL_SUPPORTED_ADMIN_KEYS
            .iter()
            .cloned()
            .collect();
        maplit::hashmap! {
            ALL_ADMIN_ACTIONS => all_admin_actions.clone(),
            HEAL_ADMIN_ACTION => all_admin_actions.clone(),
            STORAGE_INFO_ADMIN_ACTION => all_admin_actions.clone(),
            SERVER_INFO_ADMIN_ACTION => all_admin_actions.clone(),
            DATA_USAGE_INFO_ADMIN_ACTION => all_admin_actions.clone(),
            HEALTH_INFO_ADMIN_ACTION => all_admin_actions.clone(),
            BANDWIDTH_MONITOR_ACTION => all_admin_actions.clone(),
            TOP_LOCKS_ADMIN_ACTION => all_admin_actions.clone(),
            PROFILING_ADMIN_ACTION => all_admin_actions.clone(),
            TRACE_ADMIN_ACTION => all_admin_actions.clone(),
            CONSOLE_LOG_ADMIN_ACTION => all_admin_actions.clone(),
            KMS_KEY_STATUS_ADMIN_ACTION => all_admin_actions.clone(),
            SERVER_UPDATE_ADMIN_ACTION => all_admin_actions.clone(),
            SERVICE_RESTART_ADMIN_ACTION => all_admin_actions.clone(),
            SERVICE_STOP_ADMIN_ACTION => all_admin_actions.clone(),
            CONFIG_UPDATE_ADMIN_ACTION => all_admin_actions.clone(),
            CREATE_USER_ADMIN_ACTION => all_admin_actions.clone(),
            DELETE_USER_ADMIN_ACTION => all_admin_actions.clone(),
            LIST_USERS_ADMIN_ACTION => all_admin_actions.clone(),
            ENABLE_USER_ADMIN_ACTION => all_admin_actions.clone(),
            DISABLE_USER_ADMIN_ACTION => all_admin_actions.clone(),
            GET_USER_ADMIN_ACTION => all_admin_actions.clone(),
            ADD_USER_TO_GROUP_ADMIN_ACTION => all_admin_actions.clone(),
            REMOVE_USER_FROM_GROUP_ADMIN_ACTION => all_admin_actions.clone(),
            LIST_GROUPS_ADMIN_ACTION => all_admin_actions.clone(),
            ENABLE_GROUP_ADMIN_ACTION => all_admin_actions.clone(),
            DISABLE_GROUP_ADMIN_ACTION => all_admin_actions.clone(),
            CREATE_SERVICE_ACCOUNT_ADMIN_ACTION => all_admin_actions.clone(),
            UPDATE_SERVICE_ACCOUNT_ADMIN_ACTION => all_admin_actions.clone(),
            REMOVE_SERVICE_ACCOUNT_ADMIN_ACTION => all_admin_actions.clone(),
            LIST_SERVICE_ACCOUNTS_ADMIN_ACTION => all_admin_actions.clone(),

            CREATE_POLICY_ADMIN_ACTION => all_admin_actions.clone(),
            DELETE_POLICY_ADMIN_ACTION => all_admin_actions.clone(),
            GET_POLICY_ADMIN_ACTION => all_admin_actions.clone(),
            ATTACH_POLICY_ADMIN_ACTION => all_admin_actions.clone(),
            LIST_USER_POLICIES_ADMIN_ACTION => all_admin_actions.clone(),
            SET_BUCKET_QUOTA_ADMIN_ACTION => all_admin_actions.clone(),
            GET_BUCKET_QUOTA_ADMIN_ACTION => all_admin_actions.clone(),
            SET_BUCKET_TARGET_ACTION => all_admin_actions.clone(),
            GET_BUCKET_TARGET_ACTION => all_admin_actions.clone(),
            SET_TIER_ACTION => all_admin_actions.clone(),
            LIST_TIER_ACTION => all_admin_actions.clone(),
            LIST_POOLS_ACTION => all_admin_actions.clone(),
        }
    };
}

impl<'a> Valid for AdminAction<'a> {
    fn is_valid(&self) -> bool {
        SUPPORTED_ADMIN_ACTIONS.contains(self)
    }
}
