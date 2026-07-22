export interface paths {
    "/api/auth/check-email": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["check_email"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/forgot-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["forgot_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["get_current_user"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/oidc/{slug}/unlink": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["unlink_oidc_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/onboarding-state": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get current onboarding state from session */
        get: operations["onboarding_state"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/onboarding-step": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Store onboarding step in session */
        post: operations["onboarding_step"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["register"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/request-email-change": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["request_email_change"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/resend-verification": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resend_verification"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/reset-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["reset_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/setup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Store pre-registration setup data (org name, networks, seed preference) in session */
        post: operations["setup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/update": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["update_password_auth"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/verify-email": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["verify_email"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Cancel subscription
         * @description In-app cancel modal endpoint. Sets Stripe `cancel_at` to the current
         *     period end (via Stripe's `MaxPeriodEnd` sentinel), stashes the canonical
         *     Scanopy reason in subscription metadata, returns the period end so the
         *     modal can render the retention disclosure.
         */
        post: operations["cancel_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/cancel/apply-discount": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Apply the discount save offer
         * @description Applies the configured Stripe coupon to the subscription. Returns 400
         *     when `STRIPE_SAVE_OFFER_COUPON_ID` is unset.
         */
        post: operations["apply_discount_save_offer"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/change-plan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Change billing plan
         * @description Upgrades or downgrades the organization's billing plan.
         */
        post: operations["change_plan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/change-plan/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Preview plan change (shows overage counts) */
        get: operations["preview_plan_change"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/checkout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a checkout session */
        post: operations["create_checkout_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/extend-trial": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Self-serve trial extend (+7 days, once per org lifetime) */
        post: operations["extend_trial"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/finalize-payment-method": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Finalize a client-confirmed SetupIntent (set the card as default) */
        post: operations["finalize_payment_method"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/inquiry": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Submit enterprise plan inquiry
         * @description Updates Brevo contact/company with inquiry data, creates a deal, and
         *     tracks an event for automation triggers. Requires authentication to
         *     link the inquiry to an organization.
         */
        post: operations["submit_enterprise_inquiry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Pause subscription billing
         * @description Pauses billing for a 30/60/90 day window. Eligibility: rolling 6-month
         *     cooldown anchored on the org's `last_paused_at`.
         */
        post: operations["pause_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/payment-method-setup-intent": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a SetupIntent for in-app card collection (Stripe Payment Element) */
        post: operations["create_payment_method_setup_intent"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/plans": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get available billing plans */
        get: operations["get_billing_plans"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/portal": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a billing portal session */
        post: operations["create_portal_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/reactivate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Reactivate a subscription pending cancellation
         * @description Clears Stripe's scheduled-cancellation state (`cancel_at` → None).
         *     Available while `plan_status === 'pending_cancellation'`.
         */
        post: operations["reactivate_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/resume": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Resume a paused subscription
         * @description Clears Stripe pause collection and re-activates billing. Available while
         *     `plan_status === 'paused'`. The prorated pause credit is posted to the
         *     customer's Stripe balance asynchronously by the webhook arm that fires
         *     for the `pause_collection` clear — the endpoint just returns success.
         */
        post: operations["resume_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/save-offer-coupon": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Read live terms for the configured save-offer coupon
         * @description Returns the coupon's `percent_off` and `duration_in_months` so the
         *     cancel modal's Discount panel can render the offer dynamically. The
         *     payload is `null` when `STRIPE_SAVE_OFFER_COUPON_ID` is unset — the
         *     modal hides the panel in that case.
         */
        get: operations["get_save_offer_coupon"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/webhooks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Handle Stripe webhook
         * @description Internal endpoint for Stripe webhook callbacks.
         */
        post: operations["handle_webhook"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get public server configuration
         * @description Returns public configuration settings like OIDC providers, billing status, etc.
         */
        get: operations["get_public_config"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Register a new Daemon
         * @description Internal endpoint for daemon self-registration. Creates a host entry
         *     and sets up default discovery jobs for the daemon.
         */
        post: operations["register_daemon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/heartbeat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Receive daemon heartbeat (DEPRECATED - for backwards compatibility with pre-v0.14.0 daemons)
         * @description Internal endpoint for legacy daemons to send periodic heartbeats.
         *     New daemons (v0.14.0+) use the /request-work endpoint which includes heartbeat functionality.
         *     This endpoint is kept for backwards compatibility and will be removed in a future version.
         */
        post: operations["receive_heartbeat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/request-work": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Request work from server
         * @description Internal endpoint for daemons to poll for pending discovery sessions.
         *     Also updates heartbeat and returns any pending cancellation requests.
         *     Returns tuple of (next_session, should_cancel).
         */
        post: operations["receive_work_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/startup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Daemon startup handshake
         * @description Internal endpoint for daemons to report their version on startup.
         *     Updates the daemon's version and last_seen timestamp, returns server capabilities.
         */
        post: operations["daemon_startup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/update-capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update Daemon capabilities
         * @description Legacy internal endpoint for pre-0.15 daemons to report their interfaced
         *     subnets as bare ids. Modern daemons report them via the status heartbeat's
         *     `interfaced_subnets` channel; this remains functional so older daemons in a
         *     rolling deploy keep reporting (and don't 404).
         */
        post: operations["update_capabilities"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/github-stars": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get GitHub star count
         * @description Returns the current star count for the Scanopy GitHub repository.
         */
        get: operations["get_stars"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Daemon API Keys */
        get: operations["list_daemon_api_keys"];
        put?: never;
        /** Create Daemon API Key */
        post: operations["create_daemon_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete daemon_api_keys
         * @description Returns 409 Conflict if any key is currently assigned to a daemon.
         */
        post: operations["bulk_delete_daemon_api_keys"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Daemon API Keys to CSV
         * @description Export all Daemon API Keys matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_daemon_api_keys_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Daemon API Key by ID */
        get: operations["get_daemon_api_key_by_id"];
        /** Update a Daemon API Key */
        put: operations["update_daemon_api_key"];
        post?: never;
        /**
         * Delete daemon_api_key
         * @description Returns 409 Conflict if the key is currently assigned to a daemon.
         */
        delete: operations["delete_daemon_api_key"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/{id}/rotate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Rotate a Daemon API Key */
        post: operations["rotate_key_handler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get all user API keys for the current user */
        get: operations["get_all_user_api_keys"];
        put?: never;
        /** Create a new user API key */
        post: operations["create_user_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete user API keys */
        post: operations["bulk_delete_user_api_keys"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export User API Keys to CSV
         * @description Export all User API Keys matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_user_api_keys_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get a user API key by ID */
        get: operations["get_user_api_key_by_id"];
        /** Update a user API key */
        put: operations["update_user_api_key"];
        post?: never;
        /** Delete a user API key */
        delete: operations["delete_user_api_key"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/{id}/rotate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Rotate a user API key */
        post: operations["rotate_user_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Bindings */
        get: operations["list_bindings"];
        put?: never;
        /**
         * Create a new Binding
         * @description Creates a binding that associates a service with a port or interface.
         *
         *     ### Binding Types
         *
         *     - **Interface binding**: Service is present at an interface (IP address) without a specific port.
         *       Used for non-port-bound services like gateways.
         *     - **Port binding (specific ip_address)**: Service listens on a specific port on a specific interface.
         *     - **Port binding (all ip_addresses)**: Service listens on a specific port on all ip_addresses
         *       (`ip_address_id: null`).
         *
         *     ### Validation and Deduplication Rules
         *
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all ip_addresses conflicts with any interface binding for the same service.
         *     - **All-interfaces precedence**: When creating a port binding with `ip_address_id: null`,
         *       any existing specific-interface bindings for the same port are automatically removed,
         *       as they are superseded by the all-interfaces binding.
         */
        post: operations["create_binding"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Bindings */
        post: operations["bulk_delete_bindings"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Bindings to CSV
         * @description Export all Bindings matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_bindings_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Binding by ID */
        get: operations["get_binding_by_id"];
        /**
         * Update a Binding
         * @description Updates an existing binding. The same conflict detection rules from binding creation apply.
         *
         *     ## Validation Rules
         *
         *     - **Conflict detection**: The updated binding must not conflict with other bindings on the
         *       same service. Interface bindings conflict with port bindings on the same interface.
         */
        put: operations["update_binding"];
        post?: never;
        /** Delete Binding */
        delete: operations["delete_binding"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all Credentials
         * @description Returns all credentials in the authenticated user's organization.
         *     Optionally filter by type (e.g. ?type=Snmp).
         */
        get: operations["get_all_credentials"];
        put?: never;
        /**
         * Create a new Credential
         * @description Creates a credential scoped to your organization.
         */
        post: operations["create_credential"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/bulk": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk create Credentials
         * @description Creates multiple credentials in one request. Validation is atomic — if any
         *     credential has an invalid type, none are created. Individual creates are
         *     sequential, so a mid-batch DB error leaves earlier credentials committed.
         */
        post: operations["bulk_create_credentials"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Credentials */
        post: operations["bulk_delete_credentials"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Credentials to CSV
         * @description Export all Credentials matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_credentials_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get a Credential by ID */
        get: operations["get_by_id_credential"];
        /** Update Credential */
        put: operations["update_credential"];
        post?: never;
        /** Delete Credential */
        delete: operations["delete_credential"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get all daemons
         * @description Returns all daemons accessible to the user.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_daemons"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete daemons */
        post: operations["bulk_delete_daemons"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/email-install-command": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Email install command to current user */
        post: operations["email_install_command"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Daemons to CSV
         * @description Export all Daemons matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_daemons_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/provision": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Provision a daemon, or re-provision an existing one
         * @description Creates a daemon record on the server before the daemon is installed, mints an API key bound
         *     to it 1:1, and returns ready-to-run install artifacts.
         *
         *     When `daemon_id` is supplied the existing record is reused instead of creating a new one —
         *     this both re-issues install artifacts after install config changes and gives a legacy daemon
         *     (one with no bound key) a pathway to a dedicated key without losing its host, discovery jobs,
         *     or history.
         *
         *     Returns the daemon record and an API key that must be configured on the daemon.
         */
        post: operations["provision_daemon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/test-reachability": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Test reachability of a daemon URL
         * @description Performs a TCP connection test and optionally an HTTP health check
         *     to verify that a daemon URL is reachable from the server.
         */
        post: operations["test_daemon_reachability"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get daemon by ID
         * @description Returns a specific daemon with computed version status.
         */
        get: operations["get_daemon_by_id"];
        /**
         * Update daemon
         * @description Edits the server-side daemon record: its name, maintainer, tags, and — for ServerPoll —
         *     the url the server dials. Identity and server-managed fields (network, mode, host, key
         *     binding, version, liveness) are restored from the existing record by
         *     `preserve_immutable_fields`.
         */
        put: operations["update_daemon"];
        post?: never;
        /** Delete daemon */
        delete: operations["delete_daemon"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}/install-command": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Generate daemon install command
         * @description A pure, idempotent builder — it never mints or persists anything. The api key in an `install`
         *     command is a placeholder (`<API_KEY>`) the caller substitutes from the plaintext it holds; a
         *     `reconfigure` command carries no key at all. Minting is a separate mutation
         *     (`POST /provision`), so regenerating a command here (advanced-setting change, OS switch, the
         *     Details reconfigure view) never rotates the daemon's key.
         *
         *     The server derives the exact command shape from the record: DaemonPoll vs ServerPoll for the
         *     flags, and — for `install` — whether the daemon has checked in (`last_seen`) to decide between
         *     a first-install and a minimal re-key command.
         */
        get: operations["get_daemon_install_command"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}/retry-connection": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Retry connection to unreachable daemon
         * @description Resets the is_unreachable flag for a daemon that was marked unreachable
         *     due to repeated polling failures. The poller will attempt to contact
         *     the daemon again on the next cycle.
         */
        post: operations["retry_daemon_connection"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dashboard/summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get dashboard summary
         * @description Returns aggregated dashboard data including network metrics, daemon health,
         *     recent discoveries, and plan usage.
         */
        get: operations["get_dashboard_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all Dependencies
         * @description Returns all dependencies the authenticated user has access to.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_dependencies"];
        put?: never;
        /** Create a new Dependency */
        post: operations["create_dependency"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Dependencies */
        post: operations["bulk_delete_dependencies"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Dependencies to CSV
         * @description Export all Dependencies matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_dependencies_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Dependency by ID */
        get: operations["get_dependency_by_id"];
        /** Update a Dependency */
        put: operations["update_dependency"];
        post?: never;
        /** Delete Dependency */
        delete: operations["delete_dependency"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Discoveries */
        get: operations["list_discoveries"];
        put?: never;
        /** Create new Discovery */
        post: operations["create_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/active-sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get active Discovery Sessions */
        get: operations["get_active_sessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete discoveries */
        post: operations["bulk_delete_discoveries"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Discoveries to CSV
         * @description Export all Discoveries matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_discoveries_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/start-session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Start a Discovery Session */
        post: operations["start_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Discovery by ID */
        get: operations["get_discovery_by_id"];
        /** Update Discovery */
        put: operations["update_discovery"];
        post?: never;
        /** Delete discovery */
        delete: operations["delete_discovery"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{session_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Cancel a Discovery Session */
        post: operations["cancel_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{session_id}/update": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Receive discovery progress update from daemon
         * @description Internal endpoint for daemons to report discovery progress.
         */
        post: operations["receive_discovery_update"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all hosts
         * @description Returns all hosts the authenticated user has access to, with their
         *     ip_addresses, ports, and services included. Supports pagination via
         *     `limit` and `offset` query parameters, and ordering via `group_by`,
         *     `order_by`, and `order_direction`.
         */
        get: operations["get_all_hosts"];
        put?: never;
        /**
         * Create a new host
         * @description Creates a host with optional ip_addresses, ports, and services.
         *     The `source` field is automatically set to `Manual`.
         *
         *     ### Tag Validation
         *
         *     - Tags must exist and belong to your organization
         *     - Duplicate tag UUIDs are automatically deduplicated
         *     - Invalid or cross-organization tag UUIDs return a 400 error
         */
        post: operations["create_host"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete hosts
         * @description Deletes multiple hosts in a single request. The request body should be
         *     an array of host IDs to delete. Fails if any host has an associated daemon.
         */
        post: operations["bulk_delete_hosts"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Internal endpoint for daemon discovery
         * @description Used by daemons to report discovered hosts. Accepts full entities with
         *     pre-generated IDs. Uses upsert behavior to merge with existing hosts.
         *
         *     Tagged as "internal" - included in OpenAPI spec for client generation
         *     but hidden from public documentation.
         */
        post: operations["create_host_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Hosts to CSV
         * @description Export all Hosts matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_hosts_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/export/zip": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export hosts with children to ZIP
         * @description Exports all hosts matching the filter criteria along with their children
         *     (ip_addresses, ports, services, interfaces) as a ZIP archive containing
         *     separate CSV files for each entity type.
         */
        get: operations["export_hosts_zip"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/{destination_host}/consolidate/{other_host}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Consolidate hosts
         * @description Merges all ip_addresses, ports, and services from `other_host` into
         *     `destination_host`, then deletes `other_host`. Both hosts must be
         *     on the same network.
         *
         *     ### Merge Behavior
         *
         *     - **Interfaces**: Transferred to destination. If an interface with matching subnet+IP or MAC
         *       already exists on destination, bindings are remapped to use the existing interface.
         *     - **Ports**: Transferred to destination. If a port with the same number and protocol already
         *       exists, bindings are remapped to use the existing port.
         *     - **Services**: Transferred to destination with deduplication.
         *       See [upsert behavior](https://scanopy.net/docs/discovery/#upsert-behavior) for details.
         *
         *     ### Restrictions
         *
         *     - Cannot consolidate a host with itself.
         *     - Cannot consolidate a host that has a daemon - consolidate into it instead.
         */
        put: operations["consolidate_hosts"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get a host by ID
         * @description Returns a single host with its ip_addresses, ports, and services.
         */
        get: operations["get_host_by_id"];
        /**
         * Update a host
         * @description Updates host properties. Children (ip_addresses, ports, services)
         *     are managed via their own endpoints.
         *
         *     ### Tag Validation
         *
         *     - Tags must exist and belong to your organization
         *     - Duplicate tag UUIDs are automatically deduplicated
         *     - Invalid or cross-organization tag UUIDs return a 400 error
         */
        put: operations["update_host"];
        post?: never;
        /**
         * Delete a host
         * @description Prevents deletion if the host has a daemon associated with it
         */
        delete: operations["delete_host"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Interfaces */
        get: operations["list_interfaces"];
        put?: never;
        /**
         * Create a new Interface
         * @description Creates an SNMP ifTable entry for a host. These are typically created by
         *     SNMP discovery, but can also be created manually.
         */
        post: operations["create_if_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Interfaces */
        post: operations["bulk_delete_interfaces"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Interfaces to CSV
         * @description Export all Interfaces matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_interfaces_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Interface by ID */
        get: operations["get_interface_by_id"];
        /** Update an Interface */
        put: operations["update_if_entry"];
        post?: never;
        /** Delete Interface */
        delete: operations["delete_interface"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all invites */
        get: operations["get_invites"];
        put?: never;
        /** Create invite */
        post: operations["create_invite"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get an invite by ID */
        get: operations["get_invite"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites/{id}/revoke": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Revoke an invite */
        delete: operations["revoke_invite"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all IP Addresses */
        get: operations["list_ip_addresses"];
        put?: never;
        /**
         * Create a new IP address
         *     Position is automatically assigned to the end of the host's IP address list.
         */
        post: operations["create_ip_address"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete IP addresses
         *     Remaining IP addresses for affected hosts are renumbered to maintain sequential positions.
         */
        post: operations["bulk_delete_ip_addresses"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export IP Addresses to CSV
         * @description Export all IP Addresses matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_ip_addresses_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get IP Address by ID */
        get: operations["get_ip_address_by_id"];
        /**
         * Update an IP address
         *     Position must be within valid range and not conflict with other IP addresses.
         */
        put: operations["update_ip_address"];
        post?: never;
        /**
         * Delete an IP address
         *     Remaining IP addresses for the host are renumbered to maintain sequential positions.
         */
        delete: operations["delete_ip_address"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all networks */
        get: operations["get_all_networks"];
        put?: never;
        /** Create a new network */
        post: operations["create_network"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete networks */
        post: operations["bulk_delete_networks"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Networks to CSV
         * @description Export all Networks matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_networks_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get a network by ID */
        get: operations["get_by_id_network"];
        /** Update a network */
        put: operations["update_network"];
        post?: never;
        /** Delete a network */
        delete: operations["delete_network"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get the current user's organization */
        get: operations["get_organization"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/daemon-prompt-response": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Record the user's response to the daemon-install prompt so it is not shown again.
         *     Each CTA persists a distinct onboarding milestone (the org subscriber dedups); the
         *     PostHog subscriber turns these into funnel events, so no client-side telemetry is needed.
         */
        post: operations["daemon_prompt_response"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/profile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Update user profile with deferred marketing fields */
        post: operations["update_profile"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/referral-source": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Submit referral source (how did you hear about us) */
        post: operations["submit_referral_source"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Update organization name */
        put: operations["update_org_name"];
        post?: never;
        /** Delete the organization entirely, including all data and users */
        delete: operations["delete_organization"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}/populate-demo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Populate demo data (only available for demo organizations) */
        post: operations["populate_demo_data"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}/reset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Reset all organization data (delete all entities except organization and owner user) */
        post: operations["reset"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Ports */
        get: operations["list_ports"];
        put?: never;
        /** Create a new port */
        post: operations["create_port"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Ports */
        post: operations["bulk_delete_ports"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Ports to CSV
         * @description Export all Ports matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_ports_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Port by ID */
        get: operations["get_port_by_id"];
        /** Update a port */
        put: operations["update_port"];
        post?: never;
        /** Delete Port */
        delete: operations["delete_port"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all services
         * @description Returns all services the authenticated user has access to.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_services"];
        put?: never;
        /**
         * Create a new service
         * @description Creates a service with optional bindings to ip_addresses or ports.
         *     The `id`, `created_at`, `updated_at`, and `source` fields are generated server-side.
         *     Bindings are specified without `service_id` or `network_id` - these are assigned automatically.
         *
         *     ### Binding Validation Rules
         *
         *     - **Cross-host validation**: All bindings must reference ports/interfaces that belong to the
         *       service's host. Bindings referencing entities from other hosts will be rejected.
         *     - **Deduplication**: Duplicate bindings in the same request are automatically deduplicated.
         *     - **All-interfaces precedence**: If a port binding with `ip_address_id: null` (all ip_addresses)
         *       is included, any specific-interface bindings for the same port are automatically removed.
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all ip_addresses conflicts with any interface binding.
         */
        post: operations["create_service"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Services */
        post: operations["bulk_delete_services"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Services to CSV
         * @description Export all Services matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_services_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Service by ID */
        get: operations["get_service_by_id"];
        /**
         * Update a service
         * @description Updates an existing service. All binding validation rules from service creation apply here as well.
         *
         *     ## Binding Validation Rules
         *
         *     - **Cross-host validation**: All bindings must reference ports/interfaces that belong to the
         *       service's host. Bindings referencing entities from other hosts will be rejected.
         *     - **Deduplication**: Duplicate bindings are automatically deduplicated.
         *     - **All-interfaces precedence**: If a port binding with `ip_address_id: null` (all ip_addresses)
         *       is included, any specific-interface bindings for the same port are automatically removed.
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         */
        put: operations["update_service"];
        post?: never;
        /** Delete Service */
        delete: operations["delete_service"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Shares */
        get: operations["list_shares"];
        put?: never;
        /** Create a new share */
        post: operations["create_share"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Shares */
        post: operations["bulk_delete_shares"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Shares to CSV
         * @description Export all Shares matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_shares_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/public/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get share metadata
         * @description Does not include any topology data
         */
        get: operations["get_public_share_metadata"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/public/{id}/verify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Verify password for a password-protected share and return an access token.
         * @description The returned token is an HS256 JWT tied to the share's current password
         *     hash; subsequent `/topology` calls send the token instead of the raw
         *     password. Changing the share password invalidates outstanding tokens.
         */
        post: operations["verify_share_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Share by ID */
        get: operations["get_share_by_id"];
        /** Update a share */
        put: operations["update_share"];
        post?: never;
        /** Delete Share */
        delete: operations["delete_share"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snapshots": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Snapshots */
        get: operations["list_snapshots"];
        put?: never;
        /**
         * Take a snapshot of the current live topology + entity state for a network.
         *     Acquires the discovery snapshot lock, creates the snapshots row, runs
         *     close-and-clone to stamp every Snapshotable entity row with `snapshot_id`
         *     and close them. The topology subscriber inserts the snapshot's topology
         *     row off the back of the `Snapshot::Created` event.
         */
        post: operations["create_snapshot"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snapshots/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Snapshot by ID */
        get: operations["get_snapshot_by_id"];
        put?: never;
        post?: never;
        /** Delete Snapshot */
        delete: operations["delete_snapshot"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all subnets
         * @description Returns all subnets accessible to the authenticated user or daemon.
         *     Daemons can only access subnets within their assigned network.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["list_subnets"];
        put?: never;
        /** Create a new subnet */
        post: operations["create_subnet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Subnets */
        post: operations["bulk_delete_subnets"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Subnets to CSV
         * @description Export all Subnets matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_subnets_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Subnet by ID */
        get: operations["get_subnet_by_id"];
        /**
         * Update a subnet
         * @description Updates subnet properties. If the CIDR is being changed, validates that
         *     all existing ip_addresses on this subnet have IPs within the new CIDR range.
         */
        put: operations["update_subnet"];
        post?: never;
        /** Delete Subnet */
        delete: operations["delete_subnet"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all tags
         * @description Returns all tags in the authenticated user's organization.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_tags"];
        put?: never;
        /**
         * Create a new tag
         * @description Creates a tag scoped to your organization. Tag names must be unique within the organization.
         *
         *     ### Validation
         *
         *     - Name must be 1-100 characters (empty names are rejected)
         *     - Name must be unique within your organization
         */
        post: operations["create_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Set all tags for an entity
         * @description Replaces all tags on an entity with the provided list.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - All tags must exist and belong to your organization
         */
        put: operations["set_entity_tags"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign/bulk-add": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk add a tag to multiple entities
         * @description Adds a single tag to multiple entities of the same type. This is useful for batch tagging operations.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - Tag must exist and belong to your organization
         *     - Entities that already have the tag are silently skipped
         */
        post: operations["bulk_add_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign/bulk-remove": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk remove a tag from multiple entities
         * @description Removes a single tag from multiple entities of the same type.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - Entities that don't have the tag are silently skipped
         */
        post: operations["bulk_remove_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Tags */
        post: operations["bulk_delete_tags"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Tags to CSV
         * @description Export all Tags matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_tags_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Tag by ID */
        get: operations["get_tag_by_id"];
        /** Update Tag */
        put: operations["update_tag"];
        post?: never;
        /** Delete Tag */
        delete: operations["delete_tag"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get all topologies for the authenticated user's networks.
         * @description Returns both live-view rows (`snapshot_id IS NULL`) and snapshot-pinned
         *     rows. The frontend renders the live one by default and renders snapshot
         *     rows when the user picks one from the snapshots dropdown.
         */
        get: operations["get_all_topologies"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/data": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Unified entity-set endpoint for the topology view.
         * @description `?snapshot_id=<id>` resolves to the snapshot's `taken_at` and returns the
         *     as-of-T entity set; otherwise returns live entities. The frontend
         *     `TopologyTab` is the sole intended consumer.
         */
        get: operations["get_topology_data"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Topologies to CSV
         * @description Export all Topologies matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_topologies_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Topology by ID */
        get: operations["get_topology_by_id"];
        put: operations["update_topology"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/export/confluence": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Export topology as Confluence wiki markup */
        get: operations["export_confluence"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/export/mermaid": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Export topology as Mermaid flowchart */
        get: operations["export_mermaid"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all users */
        get: operations["get_all_users"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete users */
        post: operations["bulk_delete_users"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Users to CSV
         * @description Export all Users matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_users_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get user by ID */
        get: operations["get_user_by_id"];
        /** Update your own user record */
        put: operations["update_user"];
        post?: never;
        /** Delete a user */
        delete: operations["delete_user"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}/admin": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Admin update user (for changing permissions) */
        put: operations["admin_update_user"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all VLANs
         * @description Returns VLANs accessible to the authenticated user, optionally filtered by network.
         */
        get: operations["get_all_vlans"];
        put?: never;
        /**
         * Create a new VLAN
         * @description Creates a VLAN scoped to a network. VLAN numbers must be unique within a network.
         */
        post: operations["create_vlan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Vlans */
        post: operations["bulk_delete_vlans"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk upsert VLANs from discovery
         * @description Used by daemons to report discovered VLANs. Creates new VLANs or updates names.
         *     Returns the mapping of VLAN numbers to entity UUIDs for Interface construction.
         */
        post: operations["discovery_upsert_vlans"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Vlans to CSV
         * @description Export all Vlans matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_vlans_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Vlan by ID */
        get: operations["get_vlan_by_id"];
        /** Update Vlan */
        put: operations["update_vlan"];
        post?: never;
        /** Delete Vlan */
        delete: operations["delete_vlan"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/version": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get API version information */
        get: operations["get_version"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description Error response type for API errors (no data field) */
        ApiErrorResponse: {
            /** @description Machine-readable error code for i18n translation */
            code?: string | null;
            error?: string | null;
            /** @description API metadata (version info) */
            meta: components["schemas"]["ApiMeta"];
            /** @description Parameters for interpolating into the translated error message */
            params?: {
                [key: string]: unknown;
            } | null;
            success: boolean;
        };
        /**
         * @description API metadata included in all responses
         * @example {
         *       "api_version": 1,
         *       "server_version": "0.17.5"
         *     }
         */
        ApiMeta: {
            /**
             * Format: int32
             * @description API version (integer, increments on breaking changes)
             */
            api_version: number;
            /**
             * @description Server version (semver)
             * @example 0.17.5
             */
            server_version: string;
        };
        ApiResponse: {
            data?: null;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Binding: {
            /**
             * @description Association between a service and a port / interface that the service is listening on
             * @example {
             *       "created_at": "2026-07-22T18:38:10.344024Z",
             *       "first_discovery_id": null,
             *       "id": "ab218d10-9d19-4a70-b45b-b8fafb5e77bb",
             *       "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-07-22T18:38:10.344024Z",
             *       "lineage_id": null,
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *       "type": "Port",
             *       "updated_at": "2026-07-22T18:38:10.344024Z",
             *       "valid_from": "2026-07-22T18:38:10.344024Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["BindingBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_BulkDeleteResponse: {
            data?: {
                deleted_count: number;
                requested_count: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_BulkTagResponse: {
            /** @description Response for bulk tag operations */
            data?: {
                /** @description Number of entities affected */
                affected_count: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_CancelSubscriptionResponse: {
            data?: {
                /** Format: date-time */
                period_end: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ChangePlanPreview: {
            data?: {
                /** Format: int64 */
                excess_hosts: number;
                /** Format: int64 */
                excess_networks: number;
                /** Format: int64 */
                excess_seats: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Credential: {
            data?: components["schemas"]["CredentialBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Daemon: {
            data?: components["schemas"]["DaemonBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonApiKey: {
            data?: components["schemas"]["DaemonApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonApiKeyResponse: {
            data?: {
                api_key: components["schemas"]["DaemonApiKey"];
                key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonRegistrationResponse: {
            /** @description Daemon registration response from server to daemon */
            data?: {
                daemon: components["schemas"]["Daemon"];
                /** Format: uuid */
                host_id: string;
                server_capabilities?: null | components["schemas"]["ServerCapabilities"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonResponse: {
            /** @description Daemon response for UI including computed version status */
            data?: components["schemas"]["DaemonBase"] & {
                /** Format: date-time */
                created_at: string;
                /** Format: uuid */
                id: string;
                /**
                 * @description Subnets this daemon has interfaces on, loaded from the
                 *     `daemon_interfaced_subnets` junction (replaces the old
                 *     `capabilities.interfaced_subnet_ids` JSONB field).
                 */
                interfaced_subnet_ids: string[];
                /** Format: date-time */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DashboardSummary: {
            /** @description Dashboard summary response */
            data?: {
                daemons: components["schemas"]["DaemonResponse"][];
                networks: components["schemas"]["NetworkSummary"][];
                plan_usage: components["schemas"]["PlanUsage"];
                recent_discoveries: components["schemas"]["Discovery"][];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Dependency: {
            /**
             * @example {
             *       "color": "Blue",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "dependency_type": "RequestPath",
             *       "description": "HTTP/HTTPS services dependency",
             *       "edge_style": "Bezier",
             *       "id": "550e8400-e29b-41d4-a716-446655440008",
             *       "lineage_id": null,
             *       "members": {
             *         "service_ids": [],
             *         "type": "Services"
             *       },
             *       "name": "Web Services",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["DependencyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Discovery: {
            data?: components["schemas"]["DiscoveryBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** @description When true, the next scan will be a full port scan regardless of interval */
                force_full_scan?: boolean;
                /** Format: uuid */
                readonly id: string;
                /**
                 * @description Per-daemon integration targeting: which integrations (credentialed or credential-less
                 *     local) run on this daemon, and on which IPs. Delivered via the init command at
                 *     registration and editable via the discovery modal. Persistent — re-applied every scan.
                 *     This is the single home for cred↔IP targeting; it replaces the global
                 *     `credential.target_ips` (race-prone, consumed once) and the discovery modal's old
                 *     one-shot `pending_credential_ids`.
                 */
                integration_targets: components["schemas"]["IntegrationTarget"][];
                /**
                 * Format: int32
                 * @description Number of completed scans (incremented by server on session completion)
                 */
                readonly scan_count?: number;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DiscoveryUpdatePayload: {
            /** @description Progress update from daemon to server during discovery */
            data?: {
                /** Format: uuid */
                daemon_id: string;
                /**
                 * Format: uuid
                 * @description The discovery configuration this session belongs to.
                 *     Always enriched server-side; daemons do not send this field.
                 */
                discovery_id?: string | null;
                discovery_type: components["schemas"]["DiscoveryType"];
                error?: string | null;
                /** Format: int32 */
                estimated_remaining_secs?: number | null;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: int32 */
                hosts_discovered?: number | null;
                /** Format: uuid */
                network_id: string;
                phase: components["schemas"]["DiscoveryPhase"];
                /** Format: int32 */
                progress: number;
                scanned?: null | components["schemas"]["ScannedEntityIds"];
                /** Format: uuid */
                session_id: string;
                /** Format: date-time */
                started_at?: string | null;
                /**
                 * @description Non-fatal warnings for a completed run (e.g. the scan hit its time limit
                 *     and left hosts un-scanned). Unlike `error`, these do not mark the run failed.
                 */
                warnings?: string[];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_HostResponse: {
            /**
             * @description Response type for host endpoints.
             *     Includes children (ip_addresses, ports, services, interfaces).
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "credential_assignments": [],
             *       "description": "Primary web server",
             *       "hidden": false,
             *       "hostname": "web-server-01.local",
             *       "id": "550e8400-e29b-41d4-a716-446655440003",
             *       "interfaces": [
             *         {
             *           "admin_status": "Up",
             *           "cdp_address": null,
             *           "cdp_device_id": null,
             *           "cdp_platform": null,
             *           "cdp_port_id": null,
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-44665544000f",
             *           "if_alias": "Uplink to Core Switch",
             *           "if_descr": "GigabitEthernet0/1",
             *           "if_index": 1,
             *           "if_name": "Gi0/1",
             *           "if_type": 6,
             *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "lldp_chassis_id": null,
             *           "lldp_mgmt_addr": null,
             *           "lldp_port_desc": null,
             *           "lldp_port_id": null,
             *           "lldp_sys_desc": null,
             *           "lldp_sys_name": null,
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "neighbor": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "oper_status": "Up",
             *           "speed_bps": 1000000000,
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "ip_addresses": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440005",
             *           "ip_address": "192.168.1.100",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "name": "eth0",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "name": "web-server-01",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "ports": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440006",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "number": 80,
             *           "protocol": "Tcp",
             *           "type": "Http",
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "services": [
             *         {
             *           "bindings": [
             *             {
             *               "created_at": "2026-07-22T18:38:10.329979Z",
             *               "first_discovery_id": null,
             *               "id": "a990494b-da0c-45e4-9dd2-5306c1c72425",
             *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *               "last_discovery_id": null,
             *               "last_seen_at": "2026-07-22T18:38:10.329979Z",
             *               "lineage_id": null,
             *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *               "type": "Port",
             *               "updated_at": "2026-07-22T18:38:10.329979Z",
             *               "valid_from": "2026-07-22T18:38:10.329979Z",
             *               "valid_to": null
             *             }
             *           ],
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440007",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "name": "nginx",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "service_definition": "Kerberos",
             *           "source": {
             *             "type": "Manual"
             *           },
             *           "tags": [],
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null,
             *           "virtualization": null
             *         }
             *       ],
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "virtualization": null
             *     }
             */
            data?: {
                chassis_id?: string | null;
                /** Format: date-time */
                created_at: string;
                credential_assignments?: components["schemas"]["CredentialAssignment"][];
                description?: string | null;
                hidden: boolean;
                hostname?: string | null;
                /** Format: uuid */
                id: string;
                /** @description SNMP ifTable entries */
                interfaces: components["schemas"]["Interface"][];
                ip_addresses: components["schemas"]["IPAddress"][];
                management_url?: string | null;
                name: string;
                /** Format: uuid */
                network_id: string;
                ports: components["schemas"]["Port"][];
                services: components["schemas"]["Service"][];
                source: components["schemas"]["EntitySource"];
                sys_contact?: string | null;
                sys_descr?: string | null;
                sys_location?: string | null;
                sys_object_id?: string | null;
                tags: string[];
                /** Format: date-time */
                updated_at: string;
                virtualization?: null | components["schemas"]["HostVirtualization"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_IPAddress: {
            /**
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440005",
             *       "ip_address": "192.168.1.100",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "mac_address": "DE:AD:BE:EF:CA:FE",
             *       "name": "eth0",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["IPAddressBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_InstallArtifacts: {
            /**
             * @description Everything the UI needs to install (or reconfigure) a daemon, one field per install method so
             *     each is a first-class peer with its own content — no method is a special case bolted onto a
             *     list. The binary methods are ready-to-paste commands (any api key is the [`API_KEY_PLACEHOLDER`],
             *     filled in client-side); docker and msi carry their own structured content.
             */
            data?: {
                docker: components["schemas"]["DockerInstall"];
                freebsd: string;
                linux: string;
                macos: string;
                msi: components["schemas"]["MsiInstall"];
                windows: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Interface: {
            data?: components["schemas"]["InterfaceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Invite: {
            data?: components["schemas"]["InviteBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Network: {
            /**
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "credential_ids": [],
             *       "effective_stale_after_hours": 672,
             *       "id": "550e8400-e29b-41d4-a716-446655440002",
             *       "name": "Home Network",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "stale_after_hours": null,
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["NetworkBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /**
                 * Format: int64
                 * @description `stale_after_hours` with the server's default already applied.
                 *
                 *     Computed, never stored (excluded from `to_params`). Published so the
                 *     frontend derives staleness from the *same* number the digest uses rather
                 *     than re-declaring the default in TypeScript, where the two could drift
                 *     and a host could read stale in the app but current in the digest email.
                 */
                readonly effective_stale_after_hours?: number;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_OnboardingStateResponse: {
            /** @description Response from onboarding state endpoint */
            data?: {
                network?: null | components["schemas"]["OnboardingNetworkState"];
                /**
                 * Format: uuid
                 * @description Network ID from pending setup (if any)
                 */
                network_id?: string | null;
                /** @description Organization name from pending setup */
                org_name?: string | null;
                /** @description Current onboarding step (if any) */
                step?: string | null;
                use_case?: null | components["schemas"]["UseCase"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Option_SaveOfferCoupon: {
            data?: null | {
                billing_rate: components["schemas"]["BillingRate"];
                /** Format: int64 */
                duration_in_months: number;
                /** Format: date-time */
                next_renewal_at: string;
                /** Format: int64 */
                percent_off: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Organization: {
            data?: components["schemas"]["OrganizationBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Port: {
            /**
             * @description Port entity with custom serialization that flattens PortType fields.
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440006",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "number": 80,
             *       "protocol": "Tcp",
             *       "type": "Http",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["PortBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ProvisionDaemonResponse: {
            /**
             * @description Response from provisioning a daemon.
             *     Contains the daemon record and the API key (shown only once).
             *
             *     Install commands are deliberately not here — fetch them from the install-command endpoint,
             *     which builds them idempotently and fills in this key. That keeps a display-only regenerate
             *     (advanced-setting change, OS switch) from re-minting the key.
             */
            data?: {
                /** @description The created daemon record (with version status). */
                daemon: components["schemas"]["DaemonResponse"];
                /**
                 * @description The API key (plaintext) for daemon authentication.
                 *     This is shown only once - store it securely.
                 */
                daemon_api_key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_PublicConfigResponse: {
            data?: {
                billing_enabled: boolean;
                deployment_type: components["schemas"]["DeploymentType"];
                disable_password_login: boolean;
                disable_registration: boolean;
                /**
                 * @description `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
                 *     cancel modal hides the discount save-offer panel so the user
                 *     doesn't see an option the deployment can't fulfil.
                 */
                discount_save_offer_available: boolean;
                has_email_opt_in: boolean;
                has_email_service: boolean;
                has_integrated_daemon: boolean;
                /**
                 * @description Hard expiry — the drop-dead date after which the server rejects
                 *     the key. Referenced by the grace-period banner.
                 */
                license_expiry?: string | null;
                /**
                 * @description True when the license is past `intended_exp` but not yet past
                 *     the hard `exp` — the silent grace window.
                 */
                license_in_grace_period: boolean;
                /**
                 * @description User-visible expiry — the date displayed to end users under
                 *     normal operation. 7 days earlier than `license_expiry` for keys
                 *     issued after grace-period support landed.
                 */
                license_intended_expiry?: string | null;
                license_status?: string | null;
                needs_cookie_consent: boolean;
                oidc_providers: components["schemas"]["OidcProviderMetadata"][];
                /**
                 * @description True when this self-hosted instance has reached its licensed
                 *     organization cap (`included_orgs`), so new-org registration is blocked.
                 *     Always false on cloud (multi-tenant) and on unlimited-org plans.
                 */
                org_limit_reached: boolean;
                posthog_key?: string | null;
                public_url: string;
                /**
                 * Format: email
                 * @description Admin contact email to show users blocked by `org_limit_reached`,
                 *     from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
                 */
                server_admin_contact_email: string;
                /** Format: int32 */
                server_port: number;
                /**
                 * Format: int32
                 * @description `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
                 *     Frontend uses it inside the plan-comparison view to display the
                 *     effective retention for this deployment rather than the per-plan
                 *     fixture default.
                 */
                snapshot_retention_days_override?: number | null;
                /**
                 * @description Stripe publishable key, exposed so the frontend can mount Stripe
                 *     Elements (Payment Element) for in-app card collection. `None` when
                 *     billing isn't configured. Publishable keys are safe to expose to the
                 *     browser (same as `posthog_key`).
                 */
                stripe_publishable_key?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_PublicShareMetadata: {
            /** @description Public share metadata (returned without authentication) */
            data?: {
                /**
                 * @description Resolved list of available topology views for this share.
                 *     Filtered by both share configuration and data availability.
                 *     First element is the default view.
                 */
                enabled_views: components["schemas"]["TopologyView"][];
                /** Format: uuid */
                id: string;
                name: string;
                options: components["schemas"]["ShareOptions"];
                requires_password: boolean;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ServerCapabilities: {
            /** @description Server capabilities returned on startup/registration */
            data?: {
                /** @description Deprecation warnings for the daemon */
                deprecation_warnings?: components["schemas"]["DeprecationWarning"][];
                /** @description Minimum daemon version supported by this server */
                minimum_daemon_version: string;
                /** @description Server software version */
                server_version: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Service: {
            /**
             * @example {
             *       "bindings": [
             *         {
             *           "created_at": "2026-07-22T18:38:10.339196Z",
             *           "first_discovery_id": null,
             *           "id": "f3bcb1ef-9378-45b5-9410-01d995e3c88e",
             *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-07-22T18:38:10.339196Z",
             *           "lineage_id": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *           "type": "Port",
             *           "updated_at": "2026-07-22T18:38:10.339196Z",
             *           "valid_from": "2026-07-22T18:38:10.339196Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440007",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "name": "nginx",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "service_definition": "Kerberos",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null,
             *       "virtualization": null
             *     }
             */
            data?: components["schemas"]["ServiceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_SetupIntentResponse: {
            /**
             * @description Response for creating a SetupIntent — the client secret the frontend
             *     Payment Element uses to collect and confirm a card in-app.
             */
            data?: {
                client_secret: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_SetupResponse: {
            /** @description Response from setup endpoint */
            data?: {
                /** Format: uuid */
                network_id: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Share: {
            data?: components["schemas"]["ShareBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ShareAccessTokenResponse: {
            /**
             * @description Access token returned after successful password verification.
             *
             *     The token is an HS256 JWT tied to the share's `password_hash` — changing
             *     the share password implicitly invalidates all outstanding tokens.
             */
            data?: {
                access_token: string;
                /** Format: date-time */
                expires_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Snapshot: {
            data?: components["schemas"]["SnapshotBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_String: {
            data?: string;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Subnet: {
            /**
             * @example {
             *       "cidr": "192.168.1.0/24",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Local area network",
             *       "first_discovery_id": null,
             *       "id": "550e8400-e29b-41d4-a716-446655440004",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "name": "LAN",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "subnet_type": "Lan",
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["SubnetBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Tag: {
            /**
             * @example {
             *       "color": "Green",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Production environment resources",
             *       "id": "550e8400-e29b-41d4-a716-44665544000a",
             *       "is_application": false,
             *       "lineage_id": null,
             *       "name": "production",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["TagBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_TestReachabilityResponse: {
            /** @description Response from a reachability test. */
            data?: {
                /** @description Error message if not reachable */
                error?: string | null;
                /** @description Health check result (only present when check_health was true) */
                health?: boolean | null;
                /** @description Whether the TCP connection succeeded */
                reachable: boolean;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Topology: {
            data?: components["schemas"]["TopologyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_TopologyData: {
            /**
             * @description Bundle of entities + the built graph that feed the topology render, export,
             *     and share pipelines.
             *
             *     Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
             *     for either the live view (`snapshot_id = None`) or a point-in-time snapshot
             *     (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
             *     from these entities + the network's grouping options
             *     (`build_all_view_graphs`) — they are not persisted. The frontend selects the
             *     active view's slice client-side.
             */
            data?: {
                /**
                 * @description Views whose data is present in this entity set (L3/Workloads always;
                 *     L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
                 *     tags are used). The topology tab restricts a snapshot's view picker to
                 *     these — you can't set up SNMP or create app tags on a historical
                 *     snapshot — while the live view shows all views with setup prompts.
                 */
                available_views?: components["schemas"]["TopologyView"][];
                bindings: components["schemas"]["Binding"][];
                dependencies: components["schemas"]["Dependency"][];
                edges?: {
                    [key: string]: components["schemas"]["Edge"][];
                };
                hosts: components["schemas"]["Host"][];
                interfaces: components["schemas"]["Interface"][];
                ip_addresses: components["schemas"]["IPAddress"][];
                /**
                 * @description Per-view graph built on request from the entities above + grouping
                 *     options. Keyed by view so switching the active perspective is a
                 *     client-side slice selection.
                 */
                nodes?: {
                    [key: string]: components["schemas"]["Node"][];
                };
                ports: components["schemas"]["Port"][];
                services: components["schemas"]["Service"][];
                subnets: components["schemas"]["Subnet"][];
                tags: components["schemas"]["Tag"][];
                vlans: components["schemas"]["Vlan"][];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_User: {
            data?: components["schemas"]["UserBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_UserApiKey: {
            data?: components["schemas"]["UserApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_UserApiKeyResponse: {
            /**
             * @description Response for user API key creation/rotation
             *     Contains the full API key record plus the plaintext key (shown only once)
             */
            data?: {
                api_key: components["schemas"]["UserApiKey"];
                /** @description The plaintext API key - only returned once during creation or rotation */
                key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_BillingPlan: {
            data?: ((components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Community";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Free";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Starter";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Pro";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Team";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Business";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Enterprise";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Demo";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "CommercialSelfHosted";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "SelfHostedStandard";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "SelfHostedPlus";
            }))[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_Credential: {
            data?: (components["schemas"]["CredentialBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_DiscoveryUpdatePayload: {
            data?: {
                /** Format: uuid */
                daemon_id: string;
                /**
                 * Format: uuid
                 * @description The discovery configuration this session belongs to.
                 *     Always enriched server-side; daemons do not send this field.
                 */
                discovery_id?: string | null;
                discovery_type: components["schemas"]["DiscoveryType"];
                error?: string | null;
                /** Format: int32 */
                estimated_remaining_secs?: number | null;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: int32 */
                hosts_discovered?: number | null;
                /** Format: uuid */
                network_id: string;
                phase: components["schemas"]["DiscoveryPhase"];
                /** Format: int32 */
                progress: number;
                scanned?: null | components["schemas"]["ScannedEntityIds"];
                /** Format: uuid */
                session_id: string;
                /** Format: date-time */
                started_at?: string | null;
                /**
                 * @description Non-fatal warnings for a completed run (e.g. the scan hit its time limit
                 *     and left hosts un-scanned). Unlike `error`, these do not mark the run failed.
                 */
                warnings?: string[];
            }[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_Invite: {
            data?: (components["schemas"]["InviteBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_VersionInfo: {
            /** @description Version information for API compatibility checking */
            data?: {
                /**
                 * Format: int32
                 * @description Current API version (integer, increments on breaking changes)
                 */
                api_version: number;
                /** @description Minimum client version that can use this API (optional, for future use) */
                min_compatible_client?: string | null;
                /**
                 * @description Server version (semver)
                 * @example 0.12.10
                 */
                server_version: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vlan: {
            data?: components["schemas"]["VlanBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_VlanDiscoveryResponse: {
            /** @description Response for discovery upsert */
            data?: {
                /** @description Mapping of vlan_number → VLAN entity UUID */
                vlans: components["schemas"]["VlanDiscoveryResponseItem"][];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_u32: {
            /** Format: int32 */
            data?: number;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        BillingPlan: (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Community";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Free";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Starter";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Pro";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Team";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Business";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Enterprise";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Demo";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "CommercialSelfHosted";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "SelfHostedStandard";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "SelfHostedPlus";
        });
        /** @enum {string} */
        BillingRate: "Month" | "Year";
        /**
         * @description Association between a service and a port / interface that the service is listening on
         * @example {
         *       "created_at": "2026-07-22T18:38:10.330255Z",
         *       "first_discovery_id": null,
         *       "id": "7ff7c879-ad4e-42d7-8ef4-6adff4b54449",
         *       "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-07-22T18:38:10.330255Z",
         *       "lineage_id": null,
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *       "type": "Port",
         *       "updated_at": "2026-07-22T18:38:10.330255Z",
         *       "valid_from": "2026-07-22T18:38:10.330255Z",
         *       "valid_to": null
         *     }
         */
        Binding: components["schemas"]["BindingBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        /** @description The base data for a Binding entity (everything except id, created_at, updated_at) */
        BindingBase: components["schemas"]["BindingType"] & {
            /** Format: uuid */
            network_id: string;
            /** Format: uuid */
            service_id: string;
        };
        /**
         * @description Input for creating or updating a binding within a service.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the binding.
         */
        BindingInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this binding
             */
            id: string;
            /** Format: uuid */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /**
             * Format: uuid
             * @description Client-provided UUID for this binding
             */
            id: string;
            /**
             * Format: uuid
             * @description null = bind to all ip_addresses
             */
            ip_address_id?: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        /**
         * @description The type of binding - either to an interface or to a port.
         *
         *     Bindings associate a service with network resources (ip_addresses/ports) on a host.
         *
         *     ## Validation Rules
         *
         *     - All bindings must reference ports/interfaces that belong to the same host as the service.
         *     - Interface bindings conflict with port bindings on the same interface.
         *     - A port binding on all ip_addresses (`ip_address_id: null`) conflicts with any interface binding.
         *     - When a port binding with `ip_address_id: null` is created, it supersedes (removes) any
         *       existing specific-interface bindings for the same port.
         */
        BindingType: {
            /** Format: uuid */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /**
             * Format: uuid
             * @description The IP address this port binding applies to. If `null`, the binding applies to all
             *     IP addresses on the host (and supersedes specific-IP-address bindings for this port).
             */
            ip_address_id: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        BulkDeleteResponse: {
            deleted_count: number;
            requested_count: number;
        };
        /** @description Request body for bulk tag operations */
        BulkTagRequest: {
            /** @description The IDs of entities to modify */
            entity_ids: string[];
            /** @description The entity type (e.g., Host, Service, Subnet) */
            entity_type: components["schemas"]["EntityDiscriminants"];
            /**
             * Format: uuid
             * @description The tag ID to add or remove
             */
            tag_id: string;
        };
        /** @description Response for bulk tag operations */
        BulkTagResponse: {
            /** @description Number of entities affected */
            affected_count: number;
        };
        /**
         * @description Cancellation reason captured in `SubscriptionCancelled` /
         *     `CancellationInitiated` events. Mirrors the values surfaced in the
         *     in-app cancel flow (Phase 5).
         * @enum {string}
         */
        CancelReason: "too_expensive" | "missing_features" | "switched_service" | "unused" | "customer_service" | "low_quality" | "too_complex" | "other";
        CancelSubscriptionRequest: {
            comment?: string | null;
            reason_code: components["schemas"]["CancelReason"];
            save_offer_redeemed?: null | components["schemas"]["SaveOffer"];
            save_offer_shown?: components["schemas"]["SaveOffer"][];
        };
        CancelSubscriptionResponse: {
            /** Format: date-time */
            period_end: string;
        };
        ChangePlanPreview: {
            /** Format: int64 */
            excess_hosts: number;
            /** Format: int64 */
            excess_networks: number;
            /** Format: int64 */
            excess_seats: number;
        };
        ChangePlanRequest: {
            plan: components["schemas"]["BillingPlan"];
            rate: components["schemas"]["BillingRate"];
        };
        /** @description Check email availability request */
        CheckEmailRequest: {
            /** Format: email */
            email: string;
        };
        /** @enum {string} */
        Color: "Pink" | "Rose" | "Red" | "Amber" | "Orange" | "Green" | "Emerald" | "Teal" | "Cyan" | "Blue" | "Indigo" | "Purple" | "Fuchsia" | "Violet" | "Sky" | "Gray" | "Lime" | "Yellow";
        /** @enum {string} */
        ContainerType: "Subnet" | "ServiceCategory" | "Application" | "ApplicationUngrouped" | "Root" | "Host" | "NestedTag" | "NestedServiceCategory" | "Hypervisor" | "ContainerRuntime" | "Stack" | "TrunkPort" | "VLAN" | "PortOpStatus";
        /**
         * @description Input for creating a binding with a service.
         *     `service_id` and `network_id` are assigned by the server after the service is created.
         */
        CreateBindingInput: {
            /** Format: uuid */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /** Format: uuid */
            ip_address_id?: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        CreateCheckoutRequest: {
            plan: components["schemas"]["BillingPlan"];
            url: string;
        };
        /**
         * @description Request type for creating a host with its associated ip_addresses, ports, and services.
         *     Server assigns `host_id`, `network_id`, and `source` to all children.
         *     Client must provide UUIDs for all entities, enabling services to reference
         *     ip_addresses/ports by ID in the same request.
         * @example {
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "interfaces": [],
         *       "ip_addresses": [
         *         {
         *           "id": "550e8400-e29b-41d4-a716-446655440005",
         *           "ip_address": "192.168.1.100",
         *           "mac_address": "DE:AD:BE:EF:12:34",
         *           "name": "eth0",
         *           "position": 0,
         *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004"
         *         }
         *       ],
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "ports": [
         *         {
         *           "id": "550e8400-e29b-41d4-a716-446655440006",
         *           "number": 80,
         *           "protocol": "Tcp"
         *         }
         *       ],
         *       "services": [
         *         {
         *           "bindings": [
         *             {
         *               "id": "550e8400-e29b-41d4-a716-446655440009",
         *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "type": "Port"
         *             }
         *           ],
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "name": "nginx",
         *           "position": 0,
         *           "service_definition": "Kerberos",
         *           "tags": [],
         *           "virtualization": null
         *         }
         *       ],
         *       "tags": [],
         *       "virtualization": null
         *     }
         */
        CreateHostRequest: {
            chassis_id?: string | null;
            credential_assignments?: components["schemas"]["CredentialAssignment"][];
            description?: string | null;
            hidden?: boolean;
            hostname?: string | null;
            /** @description SNMP interface entries (ifTable data) - server assigns UUIDs */
            interfaces?: components["schemas"]["InterfaceInput"][];
            /** @description Interfaces to create with this host (client provides UUIDs) */
            ip_addresses?: components["schemas"]["IPAddressInput"][];
            management_url?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Ports to create with this host (client provides UUIDs) */
            ports?: components["schemas"]["PortInput"][];
            /** @description Services to create with this host (can reference ip_addresses/ports by their UUIDs) */
            services?: components["schemas"]["ServiceInput"][];
            sys_contact?: string | null;
            sys_descr?: string | null;
            sys_location?: string | null;
            sys_object_id?: string | null;
            tags: string[];
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        CreateInviteRequest: {
            /** Format: int64 */
            expiration_hours?: number | null;
            network_ids: string[];
            permissions: components["schemas"]["UserOrgPermissions"];
            send_to?: string | null;
        };
        /**
         * @description Request type for creating a service.
         *     Server assigns `id`, `created_at`, `updated_at`, and `source`.
         *     Server also assigns `service_id` and `network_id` to all bindings.
         */
        CreateServiceRequest: {
            /**
             * @description Bindings to create with the service.
             *     `service_id` and `network_id` are assigned by the server.
             */
            bindings?: components["schemas"]["CreateBindingInput"][];
            /** Format: uuid */
            host_id: string;
            name: string;
            /** Format: uuid */
            network_id: string;
            service_definition: string;
            tags: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        CreateSnapshotRequest: {
            /** Format: uuid */
            network_id: string;
        };
        CreateUpdateShareRequest: {
            share: components["schemas"]["Share"];
        };
        Credential: components["schemas"]["CredentialBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        /** @description A credential assigned to a host, optionally limited to specific ip_addresses. */
        CredentialAssignment: {
            /** Format: uuid */
            credential_id: string;
            /** @description Interface IDs to limit this credential to. None = all host ip_addresses. */
            ip_address_ids: string[] | null;
        };
        CredentialBase: {
            /**
             * @description Networks this credential is assigned to (Broadcast scope).
             *     Hydrated from the `network_credentials` junction table.
             */
            assigned_network_ids: string[];
            credential_type: components["schemas"]["CredentialType"];
            /**
             * @description Hosts this credential is assigned to (PerHost scope), with optional IP scoping.
             *     Hydrated from the `host_credentials` junction table.
             */
            host_assignments: components["schemas"]["CredentialHostAssignment"][];
            name: string;
            /** Format: uuid */
            organization_id: string;
            tags: string[];
        };
        /**
         * @description Host-keyed mirror of [`CredentialAssignment`]: a host this credential is
         *     assigned to, optionally limited to specific ip_addresses. Hydrated onto a
         *     credential from the `host_credentials` junction (PerHost scope).
         */
        CredentialHostAssignment: {
            /** Format: uuid */
            host_id: string;
            /** @description IP address IDs to limit this credential to on the host. None = all host ip_addresses. */
            ip_address_ids: string[] | null;
        };
        /** @enum {string} */
        CredentialOrderField: "created_at" | "name" | "updated_at";
        /**
         * @description Universal credential type — tagged enum stored as JSONB.
         *     Each variant represents a different credential protocol/method.
         */
        CredentialType: {
            community: components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "SnmpV1";
        } | {
            community: components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "SnmpV2c";
        } | {
            auth_password: components["schemas"]["SecretValue"];
            auth_protocol: components["schemas"]["SnmpV3AuthProtocol"];
            /** @description Optional context name (default/empty context used if unset). */
            context_name?: string | null;
            priv_password: components["schemas"]["SecretValue"];
            priv_protocol: components["schemas"]["SnmpV3PrivProtocol"];
            security_name: string;
            /** @enum {string} */
            type: "SnmpV3";
        } | {
            /** @description Optional URL path prefix (e.g. "/v1.43") */
            path?: string | null;
            /**
             * Format: int32
             * @description Port for the Docker API proxy (default 2375)
             */
            port?: number;
            ssl_cert?: null | components["schemas"]["FileOrInline"];
            ssl_chain?: null | components["schemas"]["FileOrInline"];
            ssl_key?: null | components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "DockerProxy";
        } | {
            socket_path?: string | null;
            /** @enum {string} */
            type: "DockerSocket";
        } | {
            /** @description Optional URL path prefix (e.g. "/v1.43") */
            path?: string | null;
            /**
             * Format: int32
             * @description Port for the Podman API proxy (default 2375)
             */
            port?: number;
            ssl_cert?: null | components["schemas"]["FileOrInline"];
            ssl_chain?: null | components["schemas"]["FileOrInline"];
            ssl_key?: null | components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "PodmanProxy";
        } | {
            socket_path?: string | null;
            /** @enum {string} */
            type: "PodmanSocket";
        };
        Daemon: components["schemas"]["DaemonBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DaemonApiKey: components["schemas"]["DaemonApiKeyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DaemonApiKeyBase: {
            /**
             * Format: uuid
             * @description Daemon this key is bound to 1:1, when provisioned server-side.
             *     NULL for legacy network-shared keys created before 1:1 provisioning,
             *     which resolve daemon identity from the X-Daemon-ID header instead.
             */
            readonly daemon_id?: string | null;
            /** Format: date-time */
            expires_at?: string | null;
            is_enabled?: boolean;
            readonly key: string;
            /** Format: date-time */
            readonly last_used: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            tags: string[];
        };
        DaemonApiKeyResponse: {
            api_key: components["schemas"]["DaemonApiKey"];
            key: string;
        };
        DaemonBase: {
            /**
             * Format: uuid
             * @description Foreign key to API key used for ServerPoll authentication.
             *     NULL for DaemonPoll daemons or those not yet linked to a key.
             */
            api_key_id?: string | null;
            /** Format: uuid */
            host_id: string;
            /**
             * @description Whether the daemon is unreachable (for ServerPoll circuit breaker).
             *     Set to true after repeated polling failures, reset via retry-connection endpoint.
             */
            is_unreachable?: boolean;
            /**
             * Format: date-time
             * @description Timestamp of last successful contact with daemon.
             *     NULL for provisioned ServerPoll daemons that haven't been contacted yet.
             */
            readonly last_seen?: string | null;
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Whether the daemon is on standby due to inactivity (no discovery in 30 days). */
            readonly standby?: boolean;
            /**
             * Format: date-time
             * @description Timestamp of the most recent standby → active transition. Set by
             *     `process_startup` when a restarted daemon is un-standby'd, and by
             *     the discovery auto-wake path. The nightly inactivity check skips
             *     daemons within the grace window (see `STANDBY_GRACE_PERIOD_DAYS`)
             *     to prevent the "restart → cleared → re-standby'd before discovery
             *     runs" race.
             */
            readonly standby_cleared_at?: string | null;
            tags: string[];
            /**
             * @description Address the *server* dials for a ServerPoll daemon. Editable (a daemon can move);
             *     unused and not editable for DaemonPoll, which dials out instead.
             */
            url: string;
            /**
             * Format: uuid
             * @description User responsible for maintaining this daemon
             */
            user_id: string;
            /** @description Daemon software version (semver format) */
            version?: string | null;
        };
        /**
         * @description Legacy heartbeat payload for backwards compatibility with pre-v0.14.0 daemons.
         *     Old daemons call POST /api/daemons/{id}/heartbeat with this payload.
         */
        DaemonHeartbeatPayload: {
            mode: components["schemas"]["DaemonMode"];
            name: string;
            url: string;
        };
        /**
         * @description Daemon operating mode that determines the communication pattern.
         *
         *     - **DaemonPoll** (formerly "Pull"): Daemon makes outbound connections to the server.
         *       The daemon registers itself and polls for work. Best for daemons behind NAT/firewall.
         *
         *     - **ServerPoll** (formerly "Push"): Server makes connections to the daemon.
         *       Server polls daemon for status and discovery results. Best for DMZ deployments
         *       where daemon cannot make outbound connections.
         * @enum {string}
         */
        DaemonMode: "server_poll" | "daemon_poll";
        /**
         * @description Fields that daemons can be ordered/grouped by.
         * @enum {string}
         */
        DaemonOrderField: "created_at" | "name" | "last_seen" | "updated_at" | "network_id";
        /**
         * @description Which daemon-prompt CTA the user chose.
         * @enum {string}
         */
        DaemonPromptAction: "dismissed" | "accepted";
        /** @description Request recording the user's response to the "Start Discovering Your Network" prompt. */
        DaemonPromptResponseRequest: {
            action: components["schemas"]["DaemonPromptAction"];
        };
        /** @description Daemon registration request from daemon to server */
        DaemonRegistrationRequest: {
            /**
             * @description Legacy pre-0.15 interfaced-subnet channel (deserialize-only; see
             *     [`LegacyCapabilities`]). Repopulated by the first heartbeat, so registration
             *     does not persist it.
             */
            capabilities?: components["schemas"]["LegacyCapabilities"];
            /** Format: uuid */
            daemon_id: string;
            /**
             * @description Per-daemon integration targeting from the init command (credentialed cred↔IP and
             *     credential-less local sockets). Written to this daemon's Discovery at registration so
             *     it's present before the first session dispatches. Registration assumes new-daemon →
             *     new-server, so there is no legacy bare-`credential_ids` field — bare-uuid env back-compat
             *     is handled in the daemon's env parser, never on the wire.
             */
            integration_targets?: components["schemas"]["IntegrationTarget"][];
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /**
             * @description URL is ignored by server - kept for backwards compat with old daemons.
             *     URL is only set via admin provisioning for ServerPoll daemons.
             */
            url?: string | null;
            /**
             * Format: uuid
             * @description User responsible for maintaining this daemon (from frontend install command)
             *     Optional for backwards compat with old daemons - defaults to nil UUID
             */
            user_id?: string;
            /** @description Daemon software version (optional for backwards compat with old daemons) */
            version?: string | null;
        };
        /** @description Daemon registration response from server to daemon */
        DaemonRegistrationResponse: {
            daemon: components["schemas"]["Daemon"];
            /** Format: uuid */
            host_id: string;
            server_capabilities?: null | components["schemas"]["ServerCapabilities"];
        };
        /** @description Daemon response for UI including computed version status */
        DaemonResponse: components["schemas"]["DaemonBase"] & {
            /** Format: date-time */
            created_at: string;
            /** Format: uuid */
            id: string;
            /**
             * @description Subnets this daemon has interfaces on, loaded from the
             *     `daemon_interfaced_subnets` junction (replaces the old
             *     `capabilities.interfaced_subnet_ids` JSONB field).
             */
            interfaced_subnet_ids: string[];
            /** Format: date-time */
            updated_at: string;
            /** @description Computed version status including health and warnings */
            version_status: components["schemas"]["DaemonVersionStatus"];
        };
        /** @description Sent by daemon on startup to report version */
        DaemonStartupRequest: {
            /** @description Daemon software version (semver format) */
            daemon_version: string;
        };
        /** @description Lightweight daemon status for polling responses. */
        DaemonStatus: {
            /** @description Backwards compat: pre-v0.15.0 daemons send capabilities instead of interfaced_subnets. */
            capabilities?: components["schemas"]["LegacyCapabilities"];
            /**
             * @description Subnets detected from daemon's network ip_addresses. Server resolves these
             *     via SubnetService::create (create-or-match by CIDR) to get real IDs.
             *     v0.15.0+ daemons populate this; pre-v0.15.0 daemons leave it empty.
             */
            interfaced_subnets?: components["schemas"]["Subnet"][];
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /**
             * @description Whether the daemon can accept a new discovery session.
             *     Both DaemonPoll and ServerPoll use this to avoid dispatching work to a busy daemon.
             */
            ready_for_work?: boolean;
            /**
             * @description URL is not used by server - kept for backwards compat.
             *     Server never updates daemon URL from status (URL is set during provisioning).
             */
            url?: string | null;
            /** @description Daemon software version (semver format) */
            version?: string | null;
        };
        /** @description Daemon version status including health and any warnings */
        DaemonVersionStatus: {
            has_correct_docker_volume_mount?: boolean;
            status: components["schemas"]["VersionHealthStatus"];
            supports_unified_discovery?: boolean;
            version?: string | null;
            warnings?: components["schemas"]["DeprecationWarning"][];
        };
        /** @description Dashboard summary response */
        DashboardSummary: {
            daemons: components["schemas"]["DaemonResponse"][];
            networks: components["schemas"]["NetworkSummary"][];
            plan_usage: components["schemas"]["PlanUsage"];
            recent_discoveries: components["schemas"]["Discovery"][];
        };
        /**
         * @example {
         *       "color": "Blue",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "dependency_type": "RequestPath",
         *       "description": "HTTP/HTTPS services dependency",
         *       "edge_style": "Bezier",
         *       "id": "550e8400-e29b-41d4-a716-446655440008",
         *       "lineage_id": null,
         *       "members": {
         *         "service_ids": [],
         *         "type": "Services"
         *       },
         *       "name": "Web Services",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Dependency: components["schemas"]["DependencyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        DependencyBase: {
            color: components["schemas"]["Color"];
            dependency_type: components["schemas"]["DependencyType"];
            description?: string | null;
            edge_style: components["schemas"]["EdgeStyle"];
            /** @description Members of this dependency: either service IDs or binding IDs. */
            members: components["schemas"]["DependencyMembers"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source?: components["schemas"]["EntitySource"];
            tags: string[];
        };
        /**
         * @description The members of a dependency: either service-level or binding-level.
         *     Bindings are all-or-nothing: either every position has a binding (full L3 detail)
         *     or none do (Application-level only).
         */
        DependencyMembers: {
            service_ids: string[];
            /** @enum {string} */
            type: "Services";
        } | {
            binding_ids: string[];
            /** @enum {string} */
            type: "Bindings";
        };
        /**
         * @description Fields that dependencies can be ordered/grouped by.
         * @enum {string}
         */
        DependencyOrderField: "created_at" | "name" | "dependency_type" | "updated_at" | "network_id";
        /** @enum {string} */
        DependencyType: "RequestPath" | "HubAndSpoke";
        /** @enum {string} */
        DeploymentType: "cloud" | "commercial" | "community";
        /**
         * @description Severity level for deprecation warnings
         * @enum {string}
         */
        DeprecationSeverity: "Info" | "Warning" | "Critical" | "Unknown";
        /** @description Deprecation warning for daemon version */
        DeprecationWarning: {
            message: string;
            severity: components["schemas"]["DeprecationSeverity"];
            sunset_date?: string | null;
        };
        Discovery: components["schemas"]["DiscoveryBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** @description When true, the next scan will be a full port scan regardless of interval */
            force_full_scan?: boolean;
            /** Format: uuid */
            readonly id: string;
            /**
             * @description Per-daemon integration targeting: which integrations (credentialed or credential-less
             *     local) run on this daemon, and on which IPs. Delivered via the init command at
             *     registration and editable via the discovery modal. Persistent — re-applied every scan.
             *     This is the single home for cred↔IP targeting; it replaces the global
             *     `credential.target_ips` (race-prone, consumed once) and the discovery modal's old
             *     one-shot `pending_credential_ids`.
             */
            integration_targets: components["schemas"]["IntegrationTarget"][];
            /**
             * Format: int32
             * @description Number of completed scans (incremented by server on session completion)
             */
            readonly scan_count?: number;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DiscoveryBase: {
            /** Format: uuid */
            daemon_id: string;
            discovery_type: components["schemas"]["DiscoveryType"];
            name: string;
            /** Format: uuid */
            network_id: string;
            run_type: components["schemas"]["RunType"];
            tags: string[];
        };
        /**
         * @description Request type for daemon discovery - accepts full entities with IDs.
         *     Used internally by daemons for host creation/upsert, NOT the external API.
         *     This supports the discovery workflow where daemons manage entity IDs.
         *
         *     ## Backwards compatibility (daemons < v0.16.0)
         *
         *     Pre-v0.16.0 daemons send the old field layout:
         *       - `interfaces` → IPAddress data (now `ip_addresses`)
         *       - `if_entries` → SNMP Interface data (now `interfaces`)
         *
         *     The custom deserializer detects the old layout (missing `ip_addresses` field)
         *     and remaps fields automatically. This can be removed once all daemons are ≥ v0.16.0.
         */
        DiscoveryHostRequest: {
            host: components["schemas"]["Host"];
            /** @description SNMP interface entries (ifTable data) - optional, populated when SNMP is enabled. */
            interfaces?: components["schemas"]["Interface"][];
            /**
             * @description Whether `interfaces` is a complete, authoritative ifTable. When false (a partial SNMP walk
             *     cut short by timeout/error), the server must NOT prune interfaces missing from this scan —
             *     otherwise a transient partial walk tears down the host's L2 topology (#649). Daemons that
             *     predate this field omit it; it defaults to true so their behavior is unchanged.
             */
            interfaces_complete?: boolean;
            ip_addresses: components["schemas"]["IPAddress"][];
            ports: components["schemas"]["Port"][];
            services: components["schemas"]["Service"][];
            /**
             * @description Integration-derived subnets (e.g., Docker bridge networks) — created during
             *     create_with_children after service dedup so virtualization.service_id is correct.
             */
            subnets?: components["schemas"]["Subnet"][];
        };
        /** @enum {string} */
        DiscoveryPhase: "AwaitingSnapshot" | "Queued" | "Pending" | "Starting" | "Started" | "Scanning" | "Complete" | "Failed" | "Cancelled";
        /**
         * @description Protocol that discovered the physical link between network devices
         * @enum {string}
         */
        DiscoveryProtocol: "LLDP" | "CDP";
        DiscoveryType: {
            /** Format: uuid */
            host_id: string;
            /** @enum {string} */
            type: "SelfReport";
        } | {
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /**
             * @description SNMP credentials for querying devices during discovery
             *     Server builds this mapping before initiating discovery
             */
            snmp_credentials?: Record<string, never>;
            subnet_ids: string[] | null;
            /** @enum {string} */
            type: "Network";
        } | {
            /** Format: uuid */
            host_id: string;
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /** @enum {string} */
            type: "Docker";
        } | {
            /**
             * Format: uuid
             * @description ID of the host that the daemon is running on
             */
            host_id: string;
            /** @description Fallback strategy for naming discovered hosts */
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /** @description Per-discovery scan performance settings */
            scan_settings?: components["schemas"]["ScanSettings"];
            /** @description Subnets to scan. None = scan all interfaced subnets. */
            subnet_ids: string[] | null;
            /** @enum {string} */
            type: "Unified";
        };
        /** @description Progress update from daemon to server during discovery */
        DiscoveryUpdatePayload: {
            /** Format: uuid */
            daemon_id: string;
            /**
             * Format: uuid
             * @description The discovery configuration this session belongs to.
             *     Always enriched server-side; daemons do not send this field.
             */
            discovery_id?: string | null;
            discovery_type: components["schemas"]["DiscoveryType"];
            error?: string | null;
            /** Format: int32 */
            estimated_remaining_secs?: number | null;
            /** Format: date-time */
            finished_at?: string | null;
            /** Format: int32 */
            hosts_discovered?: number | null;
            /** Format: uuid */
            network_id: string;
            phase: components["schemas"]["DiscoveryPhase"];
            /** Format: int32 */
            progress: number;
            scanned?: null | components["schemas"]["ScannedEntityIds"];
            /** Format: uuid */
            session_id: string;
            /** Format: date-time */
            started_at?: string | null;
            /**
             * @description Non-fatal warnings for a completed run (e.g. the scan hit its time limit
             *     and left hosts un-scanned). Unlike `error`, these do not mark the run failed.
             */
            warnings?: string[];
        };
        /** @description The docker install method. */
        DockerInstall: {
            /**
             * @description A ready-to-run `docker-compose.yml` for a first install. `None` for a reconfigure — the
             *     operator keeps their own compose and swaps in `env`, rather than replacing the whole file.
             */
            compose?: string | null;
            /**
             * @description The `SCANOPY_*` environment variables (`KEY=value`) this daemon is configured with. For a
             *     reconfigure these are exactly the vars that changed, so the UI can show them as a swap-in.
             */
            env: string[];
        };
        DockerSubnetVirtualization: {
            /**
             * Format: uuid
             * @description The Docker daemon service that owns this bridge network.
             *     Different Docker daemons on different hosts = distinct bridge subnets.
             */
            service_id: string;
        };
        DockerVirtualization: {
            compose_project?: string | null;
            container_id?: string | null;
            container_name?: string | null;
            /** Format: uuid */
            service_id: string;
        };
        Edge: components["schemas"]["EdgeType"] & {
            /** Format: uuid */
            id: string;
            is_multi_hop: boolean;
            label: string | null;
            /** Format: uuid */
            source: string;
            source_handle: components["schemas"]["EdgeHandle"];
            /** Format: uuid */
            target: string;
            target_handle: components["schemas"]["EdgeHandle"];
            view_config?: components["schemas"]["EdgeViewConfig"];
        };
        /**
         * @description Whether an edge is visible by default or hidden behind a toggle
         * @enum {string}
         */
        EdgeDefaultVisibility: "visible" | "hidden";
        /** @enum {string} */
        EdgeHandle: "Top" | "Bottom" | "Left" | "Right";
        /**
         * @description Controls when an edge contributes to node highlighting on selection
         * @enum {string}
         */
        EdgeHighlightBehavior: "when_visible" | "always" | "never";
        /**
         * @description Visual stroke style for an edge
         * @enum {string}
         */
        EdgeStroke: "solid" | "dashed";
        /** @enum {string} */
        EdgeStyle: "Straight" | "SmoothStep" | "Bezier";
        EdgeType: {
            /** @enum {string} */
            edge_type: "SameHost";
            /** Format: uuid */
            host_id: string;
        } | {
            /** @enum {string} */
            edge_type: "Hypervisor";
            /** Format: uuid */
            hypervisor_service_id: string;
        } | {
            /** @enum {string} */
            edge_type: "ContainerRuntime";
            /** Format: uuid */
            host_id: string;
            /** Format: uuid */
            service_id: string;
        } | {
            /** Format: uuid */
            dependency_id: string;
            /** @enum {string} */
            edge_type: "RequestPath";
            /** Format: uuid */
            source_id: string;
            /** Format: uuid */
            target_id: string;
        } | {
            /** Format: uuid */
            dependency_id: string;
            /** @enum {string} */
            edge_type: "HubAndSpoke";
            /** Format: uuid */
            source_id: string;
            /** Format: uuid */
            target_id: string;
        } | {
            /** @enum {string} */
            edge_type: "PhysicalLink";
            protocol: components["schemas"]["DiscoveryProtocol"];
            /** Format: uuid */
            source_entity_id: string;
            /** Format: uuid */
            target_entity_id: string;
        };
        /** @enum {string} */
        EdgeTypeDiscriminants: "SameHost" | "Hypervisor" | "ContainerRuntime" | "RequestPath" | "HubAndSpoke" | "PhysicalLink";
        /** @description Per-view configuration for an edge: disabled (not in this view) or active with properties */
        EdgeViewConfig: {
            /** @enum {string} */
            type: "disabled";
        } | {
            /** @description Whether ELK should use this edge for layout positioning */
            affects_layout: boolean;
            /** @description Whether the edge is shown by default or hidden behind a toggle */
            default_visibility: components["schemas"]["EdgeDefaultVisibility"];
            /** @description When this edge contributes to node highlighting on selection */
            highlight_behavior: components["schemas"]["EdgeHighlightBehavior"];
            /** @description Whether this edge should show directional animation when highlighted */
            show_directionality: boolean;
            /** @description Visual stroke style */
            stroke: components["schemas"]["EdgeStroke"];
            /** @enum {string} */
            type: "active";
            /**
             * @description Whether this edge should be elevated to target an accepting container
             *     instead of the element inside it
             */
            will_target_container: boolean;
        };
        ElementEntityType: {
            /** @enum {string} */
            element_type: "IPAddress";
            /** Format: uuid */
            ip_address_id?: string | null;
            /** Format: uuid */
            subnet_id: string;
        } | {
            /** @enum {string} */
            element_type: "Service";
        } | {
            /** @enum {string} */
            element_type: "Host";
        } | {
            /** @enum {string} */
            element_type: "Interface";
            /** Format: uuid */
            interface_id: string;
        };
        /** @description Request body for emailing an install command to the authenticated user. */
        EmailInstallCommandRequest: {
            install_command: string;
            os: string;
        };
        /**
         * @description Per-user toggles for the user-pausable email categories. Each field maps
         *     1:1 to a [`PausableCategory`]; required emails are never gated here.
         *
         *     Stored as a JSONB blob, so new categories are added as new fields rather
         *     than via migration. New fields carry `#[serde(default = "default_true")]`
         *     so a category is opted in by default if its key is absent from the stored
         *     JSON.
         */
        EmailSettings: {
            daemon_alerts?: boolean;
            discovery_digest: boolean;
            product_onboarding?: boolean;
            trial_and_usage?: boolean;
        };
        /** @description Enterprise plan inquiry request */
        EnterpriseInquiryRequest: {
            /** @description Company name */
            company: string;
            /** @description Contact email */
            email: string;
            /** @description Message/use case description */
            message: string;
            /** @description Contact name */
            name: string;
            /**
             * Format: int64
             * @description Number of networks/sites
             */
            network_count?: number | null;
            /** @description Plan type being inquired about */
            plan_type?: string | null;
            /** @description Team/company size: 1-10, 11-25, 26-50, 51-100, 101-250, 251-500, 501-1000, 1001+ */
            team_size: string;
            /** @description Urgency: immediately, 1-3 months, 3-6 months, exploring */
            urgency?: string | null;
        };
        /** @enum {string} */
        EntityDiscriminants: "Organization" | "Invite" | "Share" | "Network" | "DaemonApiKey" | "UserApiKey" | "User" | "Tag" | "Discovery" | "Daemon" | "Host" | "Service" | "Port" | "Binding" | "IPAddress" | "Interface" | "Credential" | "Subnet" | "Vlan" | "Dependency" | "Topology" | "Snapshot" | "Unknown";
        /**
         * @description How recently discovery last observed an entity.
         *
         *     Derived, never persisted — computed from `last_seen_at` against the
         *     entity's network staleness window (`Network::stale_cutoff`). Shared by the
         *     discovery digest email and the UI so a host reported stale in the digest is
         *     the same host badged stale in the inventory and topology; running two
         *     different measures let them disagree (a scan-count measure calls an entity
         *     missing after 3 scans, which is 45 minutes on one network and 3 months on
         *     another).
         *
         *     Only discovery-managed entities can be `Stale` — see
         *     [`DiscoveryTracked::is_discovery_managed`](crate::server::shared::storage::snapshot::DiscoveryTracked::is_discovery_managed).
         * @enum {string}
         */
        EntityFreshness: "new" | "current" | "stale";
        EntitySource: {
            /** @enum {string} */
            type: "Manual";
        } | {
            /** @enum {string} */
            type: "System";
        } | {
            /** @enum {string} */
            type: "Discovery";
        } | {
            details: components["schemas"]["MatchDetails"];
            /** @enum {string} */
            type: "DiscoveryWithMatch";
        } | {
            /** @enum {string} */
            type: "Unknown";
        };
        EsxiVirtualization: {
            /** Format: uuid */
            service_id: string;
            vm_id?: string | null;
            vm_name?: string | null;
        };
        /** @description Non-secret value that can be inline content or a file path on daemon host. */
        FileOrInline: {
            /** @enum {string} */
            mode: "Inline";
            value: string;
        } | {
            /** @enum {string} */
            mode: "FilePath";
            path: string;
        };
        /**
         * @description Request to finalize a client-confirmed SetupIntent (set the collected card
         *     as the customer's default payment method).
         */
        FinalizePaymentMethodRequest: {
            setup_intent_id: string;
        };
        ForgotPasswordRequest: {
            /** Format: email */
            email: string;
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "first_discovery_id": null,
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null,
         *       "virtualization": null
         *     }
         */
        Host: components["schemas"]["HostBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Discovery (historical row) that first observed this entity. Set once
             *     (post-terminal); immutable thereafter via the `IS NULL` guard in
             *     `update_discovery_fks`.
             */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /**
             * Format: uuid
             * @description Discovery (historical row) that last touched this entity. Populated
             *     post-terminal by the per-entity-service subscriber on
             *     `DiscoveryProcessed`. NULL until the first successful discovery
             *     session terminates after this row was created.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description Last successful natural-key match by daemon discovery against this
             *     live row. Refreshed every scan, regardless of field changes.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Lineage pointer on closed historical rows back to the live row whose
             *     state they capture. NULL on live rows.
             */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description SCD2: when this row version became live. Equal to `created_at` for
             *     rows that have never ridden a snapshot; advanced to the snapshot's
             *     `taken_at` for live rows after a network snapshot fires.
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description SCD2: when this row was closed by a snapshot. NULL = currently live.
             */
            readonly valid_to?: string | null;
        };
        /**
         * @description Base data for a Host entity (stored in database).
         *     Child entities (ip_addresses, ports, services) are stored in their own tables
         *     and queried by `host_id`. They are NOT stored on the host.
         */
        HostBase: {
            /** @description LLDP lldpLocChassisId - globally unique device identifier for deduplication */
            chassis_id?: string | null;
            /** @description Credential assignments for this host (hydrated from junction table). */
            credential_assignments: components["schemas"]["CredentialAssignment"][];
            description: string | null;
            hidden: boolean;
            hostname: string | null;
            /** @description URL for device management interface (manual or discovered) */
            management_url?: string | null;
            /** @description ENTITY-MIB entPhysicalMfgName - hardware manufacturer */
            manufacturer?: string | null;
            /** @description ENTITY-MIB entPhysicalModelName - hardware model */
            model?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description ENTITY-MIB entPhysicalSerialNum - hardware serial number */
            serial_number?: string | null;
            source: components["schemas"]["EntitySource"];
            /** @description SNMP sysContact.0 - admin contact info */
            sys_contact?: string | null;
            /** @description SNMP sysDescr.0 - full system description */
            sys_descr?: string | null;
            /** @description SNMP sysLocation.0 - physical location */
            sys_location?: string | null;
            /** @description SNMP sysName.0 - administratively-assigned hostname */
            sys_name?: string | null;
            /** @description SNMP sysObjectID.0 - vendor OID for device identification */
            sys_object_id?: string | null;
            tags: string[];
            virtualization: null | components["schemas"]["HostVirtualization"];
        };
        /** @enum {string} */
        HostNamingFallback: "Ip" | "BestService";
        /**
         * @description Fields that hosts can be ordered/grouped by.
         * @enum {string}
         */
        HostOrderField: "created_at" | "name" | "hostname" | "updated_at" | "virtualized_by" | "network_id" | "interface_ip" | "last_seen_at";
        /**
         * @description Response type for host endpoints.
         *     Includes children (ip_addresses, ports, services, interfaces).
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "interfaces": [
         *         {
         *           "admin_status": "Up",
         *           "cdp_address": null,
         *           "cdp_device_id": null,
         *           "cdp_platform": null,
         *           "cdp_port_id": null,
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-44665544000f",
         *           "if_alias": "Uplink to Core Switch",
         *           "if_descr": "GigabitEthernet0/1",
         *           "if_index": 1,
         *           "if_name": "Gi0/1",
         *           "if_type": 6,
         *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "lldp_chassis_id": null,
         *           "lldp_mgmt_addr": null,
         *           "lldp_port_desc": null,
         *           "lldp_port_id": null,
         *           "lldp_sys_desc": null,
         *           "lldp_sys_name": null,
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "neighbor": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "oper_status": "Up",
         *           "speed_bps": 1000000000,
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "ip_addresses": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440005",
         *           "ip_address": "192.168.1.100",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "name": "eth0",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "ports": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440006",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "number": 80,
         *           "protocol": "Tcp",
         *           "type": "Http",
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "services": [
         *         {
         *           "bindings": [
         *             {
         *               "created_at": "2026-07-22T18:38:10.329639Z",
         *               "first_discovery_id": null,
         *               "id": "c033cbc0-a3eb-4135-a2b9-ce4e16800021",
         *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "last_discovery_id": null,
         *               "last_seen_at": "2026-07-22T18:38:10.329639Z",
         *               "lineage_id": null,
         *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *               "type": "Port",
         *               "updated_at": "2026-07-22T18:38:10.329639Z",
         *               "valid_from": "2026-07-22T18:38:10.329639Z",
         *               "valid_to": null
         *             }
         *           ],
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "name": "nginx",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "service_definition": "Kerberos",
         *           "source": {
         *             "type": "Manual"
         *           },
         *           "tags": [],
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null,
         *           "virtualization": null
         *         }
         *       ],
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "virtualization": null
         *     }
         */
        HostResponse: {
            chassis_id?: string | null;
            /** Format: date-time */
            created_at: string;
            credential_assignments?: components["schemas"]["CredentialAssignment"][];
            description?: string | null;
            hidden: boolean;
            hostname?: string | null;
            /** Format: uuid */
            id: string;
            /** @description SNMP ifTable entries */
            interfaces: components["schemas"]["Interface"][];
            ip_addresses: components["schemas"]["IPAddress"][];
            management_url?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            ports: components["schemas"]["Port"][];
            services: components["schemas"]["Service"][];
            source: components["schemas"]["EntitySource"];
            sys_contact?: string | null;
            sys_descr?: string | null;
            sys_location?: string | null;
            sys_object_id?: string | null;
            tags: string[];
            /** Format: date-time */
            updated_at: string;
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        /** HostVirtualization */
        HostVirtualization: {
            details: components["schemas"]["ProxmoxVirtualization"];
            /** @enum {string} */
            type: "Proxmox";
        } | {
            details: components["schemas"]["VCenterVirtualization"];
            /** @enum {string} */
            type: "VCenter";
        } | {
            details: components["schemas"]["EsxiVirtualization"];
            /** @enum {string} */
            type: "ESXi";
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440005",
         *       "ip_address": "192.168.1.100",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "mac_address": "DE:AD:BE:EF:CA:FE",
         *       "name": "eth0",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        IPAddress: components["schemas"]["IPAddressBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        IPAddressBase: {
            /** Format: uuid */
            host_id: string;
            ip_address: string;
            /** @description MAC address discovered from ARP, SNMP, or Docker - immutable once set */
            mac_address?: string | null;
            name: string | null;
            /** Format: uuid */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this IP address in the host's IP address list (for ordering)
             */
            position?: number;
            /** Format: uuid */
            subnet_id: string;
        };
        /**
         * @description Input for creating or updating an interface.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the interface.
         */
        IPAddressInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this interface
             */
            id: string;
            ip_address: string;
            mac_address?: string | null;
            name?: string | null;
            /**
             * Format: int32
             * @description Position in the host's interface list (for ordering).
             *     If omitted on create: appends to end of list.
             *     If omitted on update: existing ip_addresses keep their positions; new ip_addresses append.
             *     Must be all specified or all omitted across all ip_addresses in the request.
             */
            position?: number | null;
            /** Format: uuid */
            subnet_id: string;
        };
        /** @description Generic wrapper that gives any rule type a stable UUID identity. */
        IdentifiedRule_ContainerRule: {
            /** Format: uuid */
            id: string;
            /**
             * @description Rules that change which containers exist and how they nest.
             *     Container titles are data-driven (subnet CIDR, host names), not user-configurable.
             */
            rule: "BySubnet" | "MergeContainerBridges" | {
                ByApplication: {
                    tag_ids?: string[];
                };
            } | "ByHost";
        };
        /** @description Generic wrapper that gives any rule type a stable UUID identity. */
        IdentifiedRule_ElementRule: {
            /** Format: uuid */
            id: string;
            /** @description Rules that organize nodes within a container into sub-groups. */
            rule: {
                ByServiceCategory: {
                    categories: components["schemas"]["ServiceCategory"][];
                    /**
                     * @description Set by the backend on the default infrastructure rule.
                     *     Frontend uses this to identify the infra container for auto-collapse.
                     */
                    readonly is_infra_rule?: boolean;
                    title?: string | null;
                };
            } | {
                ByTag: {
                    tag_ids: string[];
                    title?: string | null;
                };
            } | "ByHypervisor" | "ByContainerRuntime" | "ByStack" | "ByTrunkPort" | "ByVLAN" | "ByPortOpStatus";
        };
        /**
         * @description SNMP ifAdminStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfAdminStatus: "Up" | "Down" | "Testing";
        /**
         * @description SNMP ifOperStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfOperStatus: "Up" | "Down" | "Testing" | "Unknown" | "Dormant" | "NotPresent" | "LowerLayerDown";
        /**
         * @description Visual grouping metadata for inlined entities.
         *     Entities sharing the same `group_id` are rendered together in the element card.
         */
        InlineGroup: {
            /**
             * Format: uuid
             * @description The inlined entity's ID (e.g., service ID).
             */
            entity_id: string;
            /**
             * Format: uuid
             * @description Shared by all members of the visual group.
             */
            group_id: string;
            role: components["schemas"]["InlineGroupRole"];
        };
        /**
         * @description Role of an inlined entity within its visual group.
         * @enum {string}
         */
        InlineGroupRole: "Header" | "Member";
        /**
         * @description Everything the UI needs to install (or reconfigure) a daemon, one field per install method so
         *     each is a first-class peer with its own content — no method is a special case bolted onto a
         *     list. The binary methods are ready-to-paste commands (any api key is the [`API_KEY_PLACEHOLDER`],
         *     filled in client-side); docker and msi carry their own structured content.
         */
        InstallArtifacts: {
            docker: components["schemas"]["DockerInstall"];
            freebsd: string;
            linux: string;
            macos: string;
            msi: components["schemas"]["MsiInstall"];
            windows: string;
        };
        /**
         * @description What the caller wants the command to do — the one axis that actually varies.
         *
         *     `install` brings a daemon up (or re-keys a legacy one): it carries the api-key placeholder,
         *     fetches the binary, and spells out the connectivity + advanced config. `reconfigure` adjusts
         *     an already-installed daemon in place: no key, no fetch, just the server-held connectivity —
         *     `scanopy-daemon install` layers it over the existing `config.json`. There is no third case:
         *     re-asserting the record's (correct) values on an installed daemon is harmless, so a first
         *     install and a re-key are the same command.
         * @enum {string}
         */
        InstallCommandKind: "install" | "reconfigure";
        /**
         * @description Per-daemon integration targeting, stored on the `Discovery` entity and delivered via the
         *     init command at registration. Each entry references exactly one stored credential and says
         *     where it applies on this daemon. This is the single home for cred↔IP targeting — it replaces
         *     the global, race-prone `credential.target_ips`.
         *
         *     The variants ARE the scopes; their strum [`Target`] discriminants are the capability enum that
         *     `CredentialType::targets()` returns and validates against (single source of truth). Every
         *     target carries a real `credential_id` — there is no credential-less branch and no nil
         *     sentinel; a local socket is just a credential whose type targets only the daemon host.
         */
        IntegrationTarget: {
            /** Format: uuid */
            credential_id: string;
            /** @enum {string} */
            scope: "DaemonHost";
        } | {
            /** Format: uuid */
            credential_id: string;
            /** @enum {string} */
            scope: "Network";
        } | {
            /** Format: uuid */
            credential_id: string;
            ips: string[];
            /** @enum {string} */
            scope: "Hosts";
        };
        Interface: components["schemas"]["InterfaceBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        InterfaceBase: {
            /** @description SNMP ifAdminStatus: 1=up, 2=down, 3=testing */
            admin_status: components["schemas"]["IfAdminStatus"];
            /** @description Remote management IP from CDP (cdpCacheAddress) */
            cdp_address?: string | null;
            /** @description Remote device ID from CDP (typically hostname, locally unique) */
            cdp_device_id?: string | null;
            /** @description Remote platform from CDP (e.g., "Cisco IOS") */
            cdp_platform?: string | null;
            /** @description Remote port ID from CDP */
            cdp_port_id?: string | null;
            /**
             * @description Bridge FDB: learned MAC addresses on this switch port.
             *     Single-MAC ports can be resolved to neighbor links server-side.
             *     Multi-MAC ports indicate uplinks where LLDP/CDP is the better source.
             */
            fdb_macs?: string[] | null;
            /** Format: uuid */
            host_id: string;
            /** @description SNMP ifAlias - user-configured description */
            if_alias?: string | null;
            /** @description SNMP ifDescr - interface description (e.g., GigabitEthernet0/1) */
            if_descr: string;
            /**
             * Format: int32
             * @description SNMP ifIndex - stable identifier within device
             */
            if_index: number;
            /** @description SNMP ifName - short interface name (e.g., Gi1/0/1) */
            if_name?: string | null;
            /**
             * Format: int32
             * @description SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
             */
            if_type: number;
            /**
             * Format: uuid
             * @description FK to IPAddress entity - this port's IP assignment (must be on same host).
             *     Old daemons send this as "interface_id".
             */
            ip_address_id?: string | null;
            lldp_chassis_id?: null | components["schemas"]["LldpChassisId"];
            /** @description Remote management IP from LLDP neighbor (lldpRemManAddr) */
            lldp_mgmt_addr?: string | null;
            /** @description Remote port description from LLDP neighbor (lldpRemPortDesc) */
            lldp_port_desc?: string | null;
            lldp_port_id?: null | components["schemas"]["LldpPortId"];
            /** @description Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info */
            lldp_sys_desc?: string | null;
            /** @description Remote system name from LLDP neighbor (lldpRemSysName) */
            lldp_sys_name?: string | null;
            /** @description MAC address from SNMP ifPhysAddress - immutable once set */
            mac_address?: string | null;
            /**
             * Format: uuid
             * @description Native/untagged VLAN entity ID on this port (resolved from Q-BRIDGE dot1qPvid)
             */
            native_vlan_id?: string | null;
            neighbor?: null | components["schemas"]["Neighbor"];
            /** Format: uuid */
            network_id: string;
            /** @description SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown */
            oper_status: components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed from ifSpeed/ifHighSpeed in bits per second
             */
            speed_bps?: number | null;
            /** @description Tagged VLAN entity IDs on this port (resolved from Q-BRIDGE dot1qVlanCurrentEgressPorts) */
            vlan_ids?: string[] | null;
        };
        /**
         * @description Input for creating an SNMP interface entry (ifTable data).
         *     Used in CreateHostRequest. Server assigns UUIDs since nothing references
         *     Interface IDs at creation time (neighbor resolution is done server-side).
         */
        InterfaceInput: {
            admin_status?: null | components["schemas"]["IfAdminStatus"];
            /** @description SNMP ifAlias - user-configured description */
            if_alias?: string | null;
            /** @description SNMP ifDescr - interface description (e.g., GigabitEthernet0/1) */
            if_descr: string;
            /**
             * Format: int32
             * @description SNMP ifIndex - stable identifier within device
             */
            if_index: number;
            /**
             * Format: int32
             * @description SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
             */
            if_type?: number | null;
            /**
             * Format: uuid
             * @description Optional FK to Interface - links this SNMP port to its IP assignment
             */
            ip_address_id?: string | null;
            /** @description MAC address from SNMP ifPhysAddress */
            mac_address?: string | null;
            oper_status?: null | components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed in bits per second
             */
            speed_bps?: number | null;
        };
        Invite: components["schemas"]["InviteBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        InviteBase: {
            /** Format: uuid */
            created_by: string;
            /** Format: date-time */
            expires_at: string;
            network_ids: string[];
            /** Format: uuid */
            organization_id: string;
            permissions: components["schemas"]["UserOrgPermissions"];
            /** @description Optional email address to send the invite to */
            send_to: string | null;
            url: string;
        };
        Ixy: {
            x: number;
            y: number;
        };
        /**
         * @description Legacy inbound-only capabilities blob.
         *
         *     Pre-0.15 daemons report their interfaced subnets as bare `subnet_id`s in this
         *     `capabilities` object (they predate the `interfaced_subnets: Vec<Subnet>`
         *     heartbeat channel). It is deserialize-only: the server never stores it, never
         *     echoes it in `DaemonResponse`, and it has no `SqlValue` variant. Reported ids
         *     are routed into the `daemon_interfaced_subnets` junction (existence-filtered)
         *     so legacy daemons keep reporting interfaced subnets. ≥0.15 daemons send the
         *     `Vec<Subnet>` channel instead and leave this empty.
         */
        LegacyCapabilities: {
            interfaced_subnet_ids: string[];
        };
        /**
         * @description LLDP Chassis ID subtypes per IEEE 802.1AB.
         *
         *     The chassis ID identifies the remote device. Different network equipment
         *     may use different subtypes depending on configuration and capabilities.
         */
        LldpChassisId: {
            /** @enum {string} */
            subtype: "ChassisComponent";
            /** @description Subtype 1: Chassis component (e.g., backplane serial number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceAlias";
            /** @description Subtype 2: Interface alias (ifAlias from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "PortComponent";
            /** @description Subtype 3: Port component (e.g., backplane port number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "MacAddress";
            /** @description Subtype 4: MAC address (most common) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "NetworkAddress";
            /** @description Subtype 5: Network address (IP address stored as string) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceName";
            /** @description Subtype 6: Interface name (ifName from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "LocallyAssigned";
            /** @description Subtype 7: Locally assigned (device-specific identifier) */
            value: string;
        };
        /**
         * @description LLDP Port ID subtypes per IEEE 802.1AB.
         *
         *     The port ID identifies the specific port on the remote device.
         */
        LldpPortId: {
            /** @enum {string} */
            subtype: "InterfaceAlias";
            /** @description Subtype 1: Interface alias (ifAlias from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "PortComponent";
            /** @description Subtype 2: Port component (e.g., backplane port number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "MacAddress";
            /** @description Subtype 3: MAC address */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "NetworkAddress";
            /** @description Subtype 4: Network address (IP address stored as string) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceName";
            /** @description Subtype 5: Interface name (ifName from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "AgentCircuitId";
            /** @description Subtype 6: Agent circuit ID (used by some providers) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "LocallyAssigned";
            /** @description Subtype 7: Locally assigned (device-specific identifier) */
            value: string;
        };
        /** @description Login request from client */
        LoginRequest: {
            /** Format: email */
            email: string;
            password: string;
        };
        /** @enum {string} */
        MatchConfidence: "NotApplicable" | "Low" | "Medium" | "High" | "Certain";
        MatchDetails: {
            confidence: components["schemas"]["MatchConfidence"];
            reason: components["schemas"]["MatchReason"];
        };
        /** @description Match reason - either a simple reason string or a container with nested reasons */
        MatchReason: {
            data: string;
            /** @enum {string} */
            type: "reason";
        } | {
            /** @description Tuple of [name: string, children: MatchReason[]] */
            data: unknown[];
            /** @enum {string} */
            type: "container";
        };
        /**
         * @description The Windows MSI install method. The MSI itself is a static release asset the UI links to; only
         *     the per-daemon pre-fill data is tenant-specific.
         */
        MsiInstall: {
            /**
             * @description Filename encoding this daemon's non-secret config. Save or rename the downloaded MSI to
             *     this name to pre-fill the installer — parse-filename.js decodes it. The api key is never
             *     encoded. Renaming a signed MSI doesn't affect its signature.
             */
            filename: string;
            /**
             * @description Config keys that did not fit in `filename` (a filename is capped at 255 characters). Empty
             *     for any ordinary config. The MSI falls back to its built-in defaults for these, so the UI
             *     should tell the user to set them in the installer — the other methods carry the full config.
             */
            omitted_config_keys: string[];
        };
        /**
         * @description Resolved LLDP/CDP neighbor connection.
         *
         *     Represents the remote endpoint this port connects to, discovered via LLDP or CDP.
         *     The two variants are mutually exclusive and represent different resolution states.
         */
        Neighbor: {
            /**
             * Format: uuid
             * @description Full resolution - the specific remote port was identified
             */
            id: string;
            /** @enum {string} */
            type: "Interface";
        } | {
            /**
             * Format: uuid
             * @description Partial resolution - the remote device was identified but not the specific port
             */
            id: string;
            /** @enum {string} */
            type: "Host";
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "credential_ids": [],
         *       "effective_stale_after_hours": 672,
         *       "id": "550e8400-e29b-41d4-a716-446655440002",
         *       "name": "Home Network",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "stale_after_hours": null,
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Network: components["schemas"]["NetworkBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /**
             * Format: int64
             * @description `stale_after_hours` with the server's default already applied.
             *
             *     Computed, never stored (excluded from `to_params`). Published so the
             *     frontend derives staleness from the *same* number the digest uses rather
             *     than re-declaring the default in TypeScript, where the two could drift
             *     and a host could read stale in the app but current in the digest email.
             */
            readonly effective_stale_after_hours?: number;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        NetworkBase: {
            /** @description Credential IDs associated with this network (hydrated from junction table). */
            credential_ids: string[];
            name: string;
            /** Format: uuid */
            organization_id: string;
            /**
             * Format: int64
             * @description How long a discovery-managed entity on this network may go unobserved
             *     before it reads as stale. `None` = unset; callers resolve the effective
             *     value through [`Network::stale_after`], never by reading this directly.
             *
             *     Network-scoped because staleness is only meaningful relative to scan
             *     cadence, and cadence is a property of a network's discoveries.
             */
            stale_after_hours: number | null;
            tags: string[];
        };
        /** @description Network configuration for setup */
        NetworkSetup: {
            name: string;
        };
        /** @description Per-network summary of entity counts */
        NetworkSummary: {
            /** Format: int64 */
            daemon_count: number;
            /** Format: int64 */
            host_count: number;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: int64 */
            service_count: number;
            /** Format: int64 */
            subnet_count: number;
        };
        Node: components["schemas"]["NodeType"] & {
            header?: string | null;
            /** Format: uuid */
            id: string;
            position: components["schemas"]["Ixy"];
            size: components["schemas"]["Uxy"];
        };
        NodeType: {
            /**
             * @description Service definition ID for logo rendering (e.g. "Docker", "Proxmox VE").
             *     Used by Hypervisor and Stack subcontainers to show the service's logo.
             */
            associated_service_definition?: string | null;
            /** @description Display color name (set by graph builder from the source entity, e.g. subnet type) */
            color?: string | null;
            container_type?: components["schemas"]["ContainerType"];
            /**
             * Format: uuid
             * @description ID of the element rule that created this container (for subcontainers like NestedTag, Hypervisor, etc.)
             */
            element_rule_id?: string | null;
            /**
             * Format: uuid
             * @description The entity this container represents (e.g. host ID for Host containers,
             *     subnet ID for Subnet containers). Used for ownership mapping on the frontend.
             */
            entity_id?: string | null;
            /** @description Display icon name (set by graph builder from the source entity, e.g. subnet type) */
            icon?: string | null;
            /** @enum {string} */
            node_type: "Container";
            /** Format: uuid */
            parent_container_id?: string | null;
            /**
             * @description When true, this container accepts edges with `will_target_container`, causing
             *     them to visually attach here instead of at elements inside.
             */
            will_accept_edges?: boolean;
        } | (components["schemas"]["ElementEntityType"] & {
            /** Format: uuid */
            container_id?: string;
            /** Format: uuid */
            host_id: string;
            /**
             * @description Visual grouping metadata for services inlined on this element.
             *     Populated by element rules (e.g., Docker containers on a VM host
             *     get InlineGroups with Header/Member roles for dotted-border rendering).
             */
            inline_groups?: components["schemas"]["InlineGroup"][];
        } & {
            /** @enum {string} */
            node_type: "Element";
        });
        OidcProviderMetadata: {
            logo?: string | null;
            name: string;
            slug: string;
        };
        /** @description Network data in onboarding state response */
        OnboardingNetworkState: {
            /**
             * Format: uuid
             * @description Network ID (if created)
             */
            id?: string | null;
            /** @description Network name */
            name: string;
        };
        /** @enum {string} */
        OnboardingOperationDiscriminants: "OrgCreated" | "OnboardingModalCompleted" | "PlanSelected" | "DaemonPromptDismissed" | "DaemonPromptAccepted" | "FirstDaemonRegistered" | "FirstTopologyRebuild" | "FirstDiscoveryCompleted" | "FirstHostDiscovered" | "SecondNetworkCreated" | "FirstTagCreated" | "FirstDependencyCreated" | "FirstUserApiKeyCreated" | "FirstSnmpCredentialCreated" | "FirstApplicationTagCreated" | "FirstCredentialCreated" | "FirstSnapshotCreated" | "InviteSent" | "InviteAccepted" | "ProfileCompleted" | "ReferralSourceCompleted";
        /** @description Response from onboarding state endpoint */
        OnboardingStateResponse: {
            network?: null | components["schemas"]["OnboardingNetworkState"];
            /**
             * Format: uuid
             * @description Network ID from pending setup (if any)
             */
            network_id?: string | null;
            /** @description Organization name from pending setup */
            org_name?: string | null;
            /** @description Current onboarding step (if any) */
            step?: string | null;
            use_case?: null | components["schemas"]["UseCase"];
        };
        /** @description Request to save onboarding step */
        OnboardingStepRequest: {
            step: string;
            use_case?: null | components["schemas"]["UseCase"];
        };
        /**
         * @description Direction for ORDER BY clauses.
         * @enum {string}
         */
        OrderDirection: "asc" | "desc";
        Organization: components["schemas"]["OrganizationBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        OrganizationBase: {
            /**
             * Format: date-time
             * @description When the currently-active save-offer discount window expires. The
             *     BillingTab chip renders only while `> now()`; expiry needs no
             *     cleanup job.
             */
            readonly discount_save_offer_active_until?: string | null;
            /**
             * Format: int64
             * @description Percent off the currently-active save-offer discount applies. Read
             *     live by the BillingTab chip so a future coupon swap renders the new
             *     value without a code change.
             */
            readonly discount_save_offer_percent_off?: number | null;
            readonly has_payment_method?: boolean;
            /**
             * Format: date-time
             * @description Most recent save-offer-discount application. NULL = never. Drives the
             *     once-per-org eligibility check in `apply_discount_save_offer` and
             *     hides the Discount panel on the cancel modal for any return visit.
             */
            readonly last_discount_at?: string | null;
            /**
             * Format: date-time
             * @description Most recent downgrade event timestamp (paid→cheaper, or paid→cancelled);
             *     powers the 14-day downgrade banner.
             */
            readonly last_downgrade_at?: string | null;
            last_downgrade_from_plan?: null | components["schemas"]["BillingPlan"];
            /**
             * Format: date-time
             * @description Most recent `Paused` billing event's timestamp; powers the 6-month
             *     rolling pause cooldown.
             */
            readonly last_paused_at?: string | null;
            name: string;
            /**
             * Format: date-time
             * @description Stripe `subscription.items.data[0].current_period_end`, mirrored on
             *     every billing event that re-anchors the period (checkout, trial start
             *     / end, plan change, renewal, pause/resume, reactivate). Cleared by
             *     SubscriptionCancelled. Powers the "Next renewal on …" line in
             *     BillingPlanModal; the UI interprets the value based on plan_status
             *     (hide for paused/cancelled/past_due where the stored value can be
             *     stale or meaningless).
             */
            readonly next_renewal_at?: string | null;
            onboarding: components["schemas"]["OnboardingOperationDiscriminants"][];
            plan: null | components["schemas"]["BillingPlan"];
            plan_status: null | components["schemas"]["PlanStatus"];
            /** Format: date-time */
            readonly trial_end_date?: string | null;
            /** @description Whether the org has used its one-time trial-extend perk. */
            readonly trial_extended_used?: boolean;
            /** @description Use case selection (homelab, company, msp, other) */
            use_case?: components["schemas"]["UseCase"];
        };
        /**
         * @description API metadata for paginated list responses (pagination is always present)
         * @example {
         *       "api_version": 1,
         *       "pagination": {
         *         "has_more": true,
         *         "limit": 50,
         *         "offset": 0,
         *         "total_count": 142
         *       },
         *       "server_version": "0.17.5"
         *     }
         */
        PaginatedApiMeta: {
            /**
             * Format: int32
             * @description API version (integer, increments on breaking changes)
             */
            api_version: number;
            /** @description Pagination info */
            pagination: components["schemas"]["PaginationMeta"];
            /**
             * @description Server version (semver)
             * @example 0.17.5
             */
            server_version: string;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Credential: {
            data: (components["schemas"]["CredentialBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_DaemonResponse: {
            data: (components["schemas"]["DaemonBase"] & {
                /** Format: date-time */
                created_at: string;
                /** Format: uuid */
                id: string;
                /**
                 * @description Subnets this daemon has interfaces on, loaded from the
                 *     `daemon_interfaced_subnets` junction (replaces the old
                 *     `capabilities.interfaced_subnet_ids` JSONB field).
                 */
                interfaced_subnet_ids: string[];
                /** Format: date-time */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Dependency: {
            data: (components["schemas"]["DependencyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_HostResponse: {
            data: {
                chassis_id?: string | null;
                /** Format: date-time */
                created_at: string;
                credential_assignments?: components["schemas"]["CredentialAssignment"][];
                description?: string | null;
                hidden: boolean;
                hostname?: string | null;
                /** Format: uuid */
                id: string;
                /** @description SNMP ifTable entries */
                interfaces: components["schemas"]["Interface"][];
                ip_addresses: components["schemas"]["IPAddress"][];
                management_url?: string | null;
                name: string;
                /** Format: uuid */
                network_id: string;
                ports: components["schemas"]["Port"][];
                services: components["schemas"]["Service"][];
                source: components["schemas"]["EntitySource"];
                sys_contact?: string | null;
                sys_descr?: string | null;
                sys_location?: string | null;
                sys_object_id?: string | null;
                tags: string[];
                /** Format: date-time */
                updated_at: string;
                virtualization?: null | components["schemas"]["HostVirtualization"];
            }[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Service: {
            data: (components["schemas"]["ServiceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Subnet: {
            data: (components["schemas"]["SubnetBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Tag: {
            data: (components["schemas"]["TagBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Topology: {
            data: (components["schemas"]["TopologyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_User: {
            data: (components["schemas"]["UserBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_UserApiKey: {
            data: (components["schemas"]["UserApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Vlan: {
            data: (components["schemas"]["VlanBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly first_discovery_id?: string | null;
                /** Format: uuid */
                readonly id: string;
                /** Format: uuid */
                readonly last_discovery_id?: string | null;
                /** Format: date-time */
                readonly last_seen_at?: string;
                /** Format: uuid */
                readonly lineage_id?: string | null;
                /** Format: date-time */
                readonly updated_at: string;
                /** Format: date-time */
                readonly valid_from?: string;
                /** Format: date-time */
                readonly valid_to?: string | null;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /**
         * @description Pagination metadata returned with paginated responses.
         * @example {
         *       "has_more": true,
         *       "limit": 50,
         *       "offset": 0,
         *       "total_count": 142
         *     }
         */
        PaginationMeta: {
            /** @description Whether there are more items after this page */
            has_more: boolean;
            /**
             * Format: int32
             * @description Maximum items per page (as requested)
             */
            limit: number;
            /**
             * Format: int32
             * @description Number of items skipped
             */
            offset: number;
            /**
             * Format: int64
             * @description Total number of items matching the filter (ignoring pagination)
             */
            total_count: number;
        };
        /**
         * @description Pagination parameters that can be composed into filter queries.
         *
         *     Default behavior:
         *     - `limit`: 50 (returns up to 50 results)
         *     - `offset`: 0 (starts from the beginning)
         *     - `limit=0`: No limit (returns all results)
         *     - `limit` values above 1000 are capped to 1000
         */
        PaginationParams: {
            /**
             * Format: int32
             * @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
             */
            limit?: number | null;
            /**
             * Format: int32
             * @description Number of results to skip. Default: 0.
             */
            offset?: number | null;
        };
        /**
         * @description Pause subscription duration. The cancel modal's `RadioGroup` posts
         *     one of these enum variants verbatim — no integer parsing at the API
         *     boundary, the type is the contract.
         * @enum {string}
         */
        PauseDuration: "days30" | "days60" | "days90";
        PauseSubscriptionRequest: {
            duration_days: components["schemas"]["PauseDuration"];
        };
        PlanConfig: {
            /** Format: int64 */
            base_cents: number;
            /** Format: int64 */
            host_cents?: number | null;
            /** Format: int64 */
            included_hosts?: number | null;
            /** Format: int64 */
            included_networks?: number | null;
            /**
             * Format: int64
             * @description Organizations allowed on one self-hosted server instance. `None` =
             *     unlimited. Only enforced for self-hosted deployments (see
             *     `provision_user`); cloud stays multi-tenant regardless. Defaulted so
             *     existing stored plan JSON deserializes unchanged.
             */
            included_orgs?: number | null;
            /** Format: int64 */
            included_seats?: number | null;
            /** Format: int64 */
            network_cents?: number | null;
            rate: components["schemas"]["BillingRate"];
            /** Format: int64 */
            seat_cents?: number | null;
            /** Format: int32 */
            trial_days: number;
        };
        /**
         * @description Derived subscription status — our domain enum, never Stripe's raw status.
         *     Stripe webhook events map to typed `BillingOperation` variants at reception
         *     (in `billing/service.rs`); each variant deterministically implies a
         *     `PlanStatus` for downstream feature gates via
         *     `BillingOperation::implied_status`.
         *
         *     `FromStr` is derived (via strum) so the storage layer can round-trip a
         *     snake_case `text` column back into the typed value; `ToSchema` exposes
         *     the enum as a stricter string union in the generated OpenAPI schema so
         *     the frontend's `org.plan_status === 'paused'` comparisons are
         *     compile-checked against the canonical variant list.
         * @enum {string}
         */
        PlanStatus: "active" | "trialing" | "past_due" | "paused" | "pending_cancellation" | "cancelled";
        /** @description Plan usage limits and current counts */
        PlanUsage: {
            /** Format: int64 */
            host_count: number;
            /** Format: int64 */
            host_limit?: number | null;
            /** Format: int64 */
            network_count: number;
            /** Format: int64 */
            network_limit?: number | null;
            /** Format: int64 */
            seat_count: number;
            /** Format: int64 */
            seat_limit?: number | null;
        };
        PodmanSubnetVirtualization: {
            /**
             * Format: uuid
             * @description The Podman daemon service that owns this bridge network.
             *     Different Podman daemons on different hosts = distinct bridge subnets.
             */
            service_id: string;
        };
        PodmanVirtualization: {
            compose_project?: string | null;
            container_id?: string | null;
            container_name?: string | null;
            /** Format: uuid */
            service_id: string;
        };
        /**
         * @description Port entity with custom serialization that flattens PortType fields.
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440006",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "number": 80,
         *       "protocol": "Tcp",
         *       "type": "Http",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Port: components["schemas"]["PortBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        /** @description The base data for a Port entity (everything except id, created_at, updated_at) */
        PortBase: components["schemas"]["PortType"] & {
            /** Format: uuid */
            host_id: string;
            /** Format: uuid */
            network_id: string;
        };
        /**
         * @description Input for creating or updating a port.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the port.
         */
        PortInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this port
             */
            id: string;
            /**
             * Format: int32
             * @description Port number (1-65535)
             */
            number: number;
            /** @description Transport protocol (Tcp or Udp) */
            protocol: components["schemas"]["TransportProtocol"];
        };
        /** @description Port type with number, protocol, and optional type identifier */
        PortType: {
            number: number;
            /** @enum {string} */
            protocol: "Udp" | "Tcp";
            /** @description Auto-derived from number+protocol; optional on create */
            type?: string;
        };
        /** @description Request to update user profile (deferred marketing fields) */
        ProfileUpdateRequest: {
            company_size?: string | null;
            job_title?: string | null;
        };
        /**
         * @description Request to pre-provision a daemon (either mode) before it is installed.
         *     This creates the daemon record + its 1:1 API key on the server so the install
         *     command shrinks to two flags.
         */
        ProvisionDaemonRequest: {
            /**
             * Format: uuid
             * @description Mint a fresh 1:1 key for this existing daemon instead of creating a new record,
             *     keeping its host, discovery jobs and history. Used to give a legacy daemon (no bound
             *     key) a dedicated one. When set, `name`/`network_id`/`mode`/`url` are ignored — those
             *     come from the existing record.
             *
             *     Only accepted for a daemon that has never checked in or has no bound key; a live
             *     provisioned daemon is refused, since it has no way to learn the new key.
             *
             *     Note: install commands are not generated here — call the install-command endpoint,
             *     which builds them idempotently and fills in the key this response returns.
             */
            daemon_id?: string | null;
            /**
             * @description How the daemon communicates with the server. Defaults to DaemonPoll
             *     (the daemon dials out) for forward-compat with older clients.
             */
            mode?: components["schemas"]["DaemonMode"];
            /**
             * @description Human-readable name for the daemon. Required unless `daemon_id` is set, in which case
             *     the existing record's name is kept.
             */
            name?: string | null;
            /**
             * Format: uuid
             * @description Network this daemon will be associated with. Required unless `daemon_id` is set, in
             *     which case the existing record's network is kept.
             */
            network_id?: string | null;
            /**
             * @description Credential/integration references to seed onto the daemon's first
             *     discovery run. References only — never secret material. Empty by default.
             */
            seed_credential_refs?: components["schemas"]["IntegrationTarget"][];
            /**
             * @description Reachable URL where the *server* can dial the daemon. Required for
             *     ServerPoll, unused for DaemonPoll (the daemon dials out instead).
             */
            url?: string | null;
        };
        /**
         * @description Response from provisioning a daemon.
         *     Contains the daemon record and the API key (shown only once).
         *
         *     Install commands are deliberately not here — fetch them from the install-command endpoint,
         *     which builds them idempotently and fills in this key. That keeps a display-only regenerate
         *     (advanced-setting change, OS switch) from re-minting the key.
         */
        ProvisionDaemonResponse: {
            /** @description The created daemon record (with version status). */
            daemon: components["schemas"]["DaemonResponse"];
            /**
             * @description The API key (plaintext) for daemon authentication.
             *     This is shown only once - store it securely.
             */
            daemon_api_key: string;
        };
        ProxmoxVirtualization: {
            /** Format: uuid */
            service_id: string;
            vm_id?: string | null;
            vm_name?: string | null;
        };
        PublicConfigResponse: {
            billing_enabled: boolean;
            deployment_type: components["schemas"]["DeploymentType"];
            disable_password_login: boolean;
            disable_registration: boolean;
            /**
             * @description `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
             *     cancel modal hides the discount save-offer panel so the user
             *     doesn't see an option the deployment can't fulfil.
             */
            discount_save_offer_available: boolean;
            has_email_opt_in: boolean;
            has_email_service: boolean;
            has_integrated_daemon: boolean;
            /**
             * @description Hard expiry — the drop-dead date after which the server rejects
             *     the key. Referenced by the grace-period banner.
             */
            license_expiry?: string | null;
            /**
             * @description True when the license is past `intended_exp` but not yet past
             *     the hard `exp` — the silent grace window.
             */
            license_in_grace_period: boolean;
            /**
             * @description User-visible expiry — the date displayed to end users under
             *     normal operation. 7 days earlier than `license_expiry` for keys
             *     issued after grace-period support landed.
             */
            license_intended_expiry?: string | null;
            license_status?: string | null;
            needs_cookie_consent: boolean;
            oidc_providers: components["schemas"]["OidcProviderMetadata"][];
            /**
             * @description True when this self-hosted instance has reached its licensed
             *     organization cap (`included_orgs`), so new-org registration is blocked.
             *     Always false on cloud (multi-tenant) and on unlimited-org plans.
             */
            org_limit_reached: boolean;
            posthog_key?: string | null;
            public_url: string;
            /**
             * Format: email
             * @description Admin contact email to show users blocked by `org_limit_reached`,
             *     from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
             */
            server_admin_contact_email: string;
            /** Format: int32 */
            server_port: number;
            /**
             * Format: int32
             * @description `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
             *     Frontend uses it inside the plan-comparison view to display the
             *     effective retention for this deployment rather than the per-plan
             *     fixture default.
             */
            snapshot_retention_days_override?: number | null;
            /**
             * @description Stripe publishable key, exposed so the frontend can mount Stripe
             *     Elements (Payment Element) for in-app card collection. `None` when
             *     billing isn't configured. Publishable keys are safe to expose to the
             *     browser (same as `posthog_key`).
             */
            stripe_publishable_key?: string | null;
        };
        /** @description Public share metadata (returned without authentication) */
        PublicShareMetadata: {
            /**
             * @description Resolved list of available topology views for this share.
             *     Filtered by both share configuration and data availability.
             *     First element is the default view.
             */
            enabled_views: components["schemas"]["TopologyView"][];
            /** Format: uuid */
            id: string;
            name: string;
            options: components["schemas"]["ShareOptions"];
            requires_password: boolean;
        };
        /** @description Request to submit referral source */
        ReferralSourceRequest: {
            referral_source: string;
            referral_source_other?: string | null;
        };
        /** @description Registration request from client */
        RegisterRequest: {
            /** @description Honeypot field for bot detection */
            company_url?: string | null;
            /** Format: email */
            email: string;
            marketing_opt_in?: boolean;
            password: string;
            terms_accepted: boolean;
        };
        RequestEmailChangeRequest: {
            /**
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users.
             */
            current_password?: string | null;
            /** Format: email */
            new_email: string;
        };
        /** @description Request to resend verification email */
        ResendVerificationRequest: {
            /** Format: email */
            email: string;
        };
        ResetPasswordRequest: {
            password: string;
            token: string;
        };
        RunType: {
            cron_schedule: string;
            enabled: boolean;
            /** Format: date-time */
            readonly last_run?: string | null;
            /** @description IANA timezone for cron evaluation, e.g. "America/New_York". None = UTC. */
            timezone?: string | null;
            /** @enum {string} */
            type: "Scheduled";
        } | {
            results: components["schemas"]["DiscoveryUpdatePayload"];
            /** @enum {string} */
            type: "Historical";
        } | {
            /** Format: date-time */
            readonly last_run?: string | null;
            /** @enum {string} */
            type: "AdHoc";
        };
        /**
         * @description Save-offer choices presented during in-app cancellation (Phase 5).
         * @enum {string}
         */
        SaveOffer: "pause" | "discount" | "downgrade";
        /**
         * @description Live terms for the configured save-offer coupon, read directly from
         *     Stripe. Used by the cancel modal's Discount panel to render the offer
         *     dynamically instead of hard-coding the percent/duration.
         *
         *     Only returned when the coupon would actually catch the user's next
         *     invoice — i.e. `next_renewal_at` falls within the coupon's `duration_in_months`
         *     window. Yearly subscribers partway through a cycle whose next renewal
         *     lands after the coupon's window get `None` from the endpoint and the
         *     cancel modal's Discount panel doesn't render.
         *
         *     `billing_rate` lets the frontend pick monthly vs yearly copy: a monthly
         *     subscriber thinks in terms of "N months of discount"; a yearly subscriber
         *     thinks in terms of "my next renewal on {date}."
         */
        SaveOfferCoupon: {
            billing_rate: components["schemas"]["BillingRate"];
            /** Format: int64 */
            duration_in_months: number;
            /** Format: date-time */
            next_renewal_at: string;
            /** Format: int64 */
            percent_off: number;
        };
        /**
         * @description Scan performance settings. Lives on the discovery entity.
         *     Numeric fields are `Option<T>` — `None` means "use daemon default".
         *     The daemon unwraps with defaults at point of use.
         */
        ScanSettings: {
            /**
             * Format: int32
             * @description ARP packets per second (default: 50)
             */
            arp_rate_pps?: number | null;
            /**
             * Format: int32
             * @description ARP retry rounds for non-responsive targets (default: 2 = 3 total attempts)
             */
            arp_retries?: number | null;
            /**
             * Format: int32
             * @description ARP scan cutoff prefix. Interfaced subnets larger than this prefix are
             *     truncated to this many IPs. Default: 15 (= /15, ~131K IPs).
             *     Lower values scan more IPs — increase arp_rate_pps accordingly.
             */
            arp_scan_cutoff?: number | null;
            /**
             * Format: int32
             * @description Run a full 65k port scan every N scans. Other scans use a light port set.
             *     Default: 3. Value of 0 means never full scan. Value of 1 means every scan is full.
             */
            full_scan_interval?: number | null;
            /**
             * @description Whether this specific scan run should do a full 65k port scan.
             *     Set by the server before dispatching to the daemon — not user-configurable.
             */
            is_full_scan?: boolean;
            /**
             * Format: int32
             * @description Hard ceiling on how long a single discovery run may take, in seconds
             *     (default: 21600 = 6h). When hit, the run force-completes and any hosts
             *     still queued are left un-scanned until the next run. Raise this for very
             *     large networks that legitimately need more than the default window.
             */
            max_discovery_duration?: number | null;
            /** @description Ports scanned concurrently per host (default: 200, clamped 16-1000) */
            port_scan_batch_size?: number | null;
            /**
             * @description Whether to probe raw-socket ports 9100-9107 (default: false).
             *     Disabled by default to prevent ghost printing on JetDirect printers.
             */
            probe_raw_socket_ports?: boolean;
            /**
             * Format: int32
             * @description Port scan probes per second (default: 500)
             */
            scan_rate_pps?: number | null;
            /** @description On Windows, use Npcap broadcast ARP instead of SendARP (default: false) */
            use_npcap_arp?: boolean;
        };
        /**
         * @description Canonical IDs of entities scanned in a discovery session.
         *
         *     Populated daemon-side at terminal phase from `EntityBuffer`'s `Created`
         *     entries. Travels with the terminal `DiscoveryUpdatePayload` to the server,
         *     rides the in-memory `EntityOperation::Created` event published for the
         *     historical Discovery row (the event scope carries `Entity::Discovery` with
         *     the full struct, including `run_type::Historical { results }`), then is
         *     stripped before persisting into the historical Discovery row's JSONB (see
         *     the `SqlValue::RunType` bind_value handler in
         *     `backend/src/server/shared/storage/generic.rs`). Per-entity-service
         *     subscribers extract `results.scanned` from the in-memory event and call
         *     `DiscoveryFkUpdater::update_discovery_fks` to backfill
         *     `last_discovery_id` / `first_discovery_id` on the matched rows.
         *
         *     Naming: `scanned_*` because the daemon scans entities — some submissions
         *     match existing rows (refresh), others insert new rows. Both populate the
         *     EntityBuffer with canonical (server-assigned) IDs.
         */
        ScannedEntityIds: {
            binding_ids?: string[];
            host_ids?: string[];
            interface_ids?: string[];
            ip_address_ids?: string[];
            port_ids?: string[];
            service_ids?: string[];
            subnet_ids?: string[];
            vlan_ids?: string[];
        };
        /** @description Secret value that can be either inline content or a file path on the daemon host. */
        SecretValue: {
            /** @enum {string} */
            mode: "Inline";
            value: string;
        } | {
            /** @enum {string} */
            mode: "FilePath";
            path: string;
        };
        /** @description Server capabilities returned on startup/registration */
        ServerCapabilities: {
            /** @description Deprecation warnings for the daemon */
            deprecation_warnings?: components["schemas"]["DeprecationWarning"][];
            /** @description Minimum daemon version supported by this server */
            minimum_daemon_version: string;
            /** @description Server software version */
            server_version: string;
        };
        /**
         * @example {
         *       "bindings": [
         *         {
         *           "created_at": "2026-07-22T18:38:10.330193Z",
         *           "first_discovery_id": null,
         *           "id": "a84c230f-9b22-4005-9af6-d595b415ebe2",
         *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-07-22T18:38:10.330193Z",
         *           "lineage_id": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *           "type": "Port",
         *           "updated_at": "2026-07-22T18:38:10.330193Z",
         *           "valid_from": "2026-07-22T18:38:10.330193Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440007",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "nginx",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "service_definition": "Kerberos",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null,
         *       "virtualization": null
         *     }
         */
        Service: components["schemas"]["ServiceBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        ServiceBase: {
            bindings: components["schemas"]["Binding"][];
            /** Format: uuid */
            host_id: string;
            name: string;
            /** Format: uuid */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this service in the host's service list (for ordering)
             */
            position: number;
            service_definition: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            tags: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        /** @enum {string} */
        ServiceCategory: "NetworkCore" | "NetworkAccess" | "NetworkAppliance" | "RemoteAccess" | "Storage" | "Backup" | "Media" | "HomeAutomation" | "Hypervisor" | "ContainerRuntime" | "Container" | "Orchestrator" | "DNS" | "VPN" | "Monitoring" | "AdBlock" | "ReverseProxy" | "Workstation" | "Mobile" | "IoT" | "Printer" | "Database" | "Development" | "Dashboard" | "MessageQueue" | "IdentityAndAccess" | "Integration" | "Office" | "ProjectManagement" | "Messaging" | "Conferencing" | "Telephony" | "Email" | "Publishing" | "Unknown" | "Custom" | "Scanopy" | "OpenPorts";
        /**
         * @description Input for creating or updating a service.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the service.
         */
        ServiceInput: {
            /** @description Bindings that associate this service with ports/interfaces */
            bindings?: components["schemas"]["BindingInput"][];
            /**
             * Format: uuid
             * @description Client-provided UUID for this service
             */
            id: string;
            /** @description Display name for this service */
            name: string;
            /**
             * Format: int32
             * @description Position in the host's service list (for ordering).
             *     If omitted on create: appends to end of list.
             *     If omitted on update: existing services keep their positions; new services append.
             *     Must be all specified or all omitted across all services in the request.
             */
            position?: number | null;
            /** @description Service definition ID (e.g., "Nginx", "PostgreSQL") */
            service_definition: string;
            /** @description Tags for categorization */
            tags?: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        /**
         * @description Fields that services can be ordered/grouped by.
         * @enum {string}
         */
        ServiceOrderField: "created_at" | "name" | "updated_at" | "host" | "network_id" | "position" | "last_seen_at";
        /** ServiceVirtualization */
        ServiceVirtualization: {
            details: components["schemas"]["DockerVirtualization"];
            /** @enum {string} */
            type: "Docker";
        } | {
            details: components["schemas"]["PodmanVirtualization"];
            /** @enum {string} */
            type: "Podman";
        };
        /** @description Request body for setting all tags on an entity */
        SetTagsRequest: {
            /**
             * Format: uuid
             * @description The entity ID
             */
            entity_id: string;
            /** @description The entity type (e.g., Host, Service, Subnet) */
            entity_type: components["schemas"]["EntityDiscriminants"];
            /** @description The new list of tag IDs */
            tag_ids: string[];
        };
        /**
         * @description Response for creating a SetupIntent — the client secret the frontend
         *     Payment Element uses to collect and confirm a card in-app.
         */
        SetupIntentResponse: {
            client_secret: string;
        };
        /** @description Setup request for pre-registration org/network configuration */
        SetupRequest: {
            network: components["schemas"]["NetworkSetup"];
            organization_name: string;
        };
        /** @description Response from setup endpoint */
        SetupResponse: {
            /** Format: uuid */
            network_id: string;
        };
        Share: components["schemas"]["ShareBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        /**
         * @description Access token returned after successful password verification.
         *
         *     The token is an HS256 JWT tied to the share's `password_hash` — changing
         *     the share password implicitly invalidates all outstanding tokens.
         */
        ShareAccessTokenResponse: {
            access_token: string;
            /** Format: date-time */
            expires_at: string;
        };
        ShareBase: {
            allowed_domains: string[] | null;
            /** Format: uuid */
            created_by: string;
            /**
             * @description Which topology views are enabled for this share.
             *     None = all views (subject to data availability). Some(list) = only these views in order.
             *     First element is the default view shown on load.
             */
            enabled_views: components["schemas"]["TopologyView"][] | null;
            /** Format: date-time */
            expires_at: string | null;
            is_enabled: boolean;
            name: string;
            /** Format: uuid */
            network_id: string;
            options: components["schemas"]["ShareOptions"];
            /**
             * @description Plaintext password on ingest; redacted sentinel (`"********"`) or `None` on egress.
             *     Never stored — `password_hash` is the DB column. Wrapped in `SecretString` so
             *     `Debug`/logging shows `[REDACTED]` during the window between request
             *     deserialization and hashing.
             */
            password?: string | null;
            /** Format: uuid */
            topology_id: string;
        };
        /** @description Share display options */
        ShareOptions: {
            show_export_button: boolean;
            show_inspect_panel: boolean;
            show_minimap: boolean;
            show_zoom_controls: boolean;
        };
        Snapshot: components["schemas"]["SnapshotBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        SnapshotBase: {
            /** Format: uuid */
            created_by_user_id?: string | null;
            /** Format: uuid */
            network_id: string;
            /** Format: date-time */
            taken_at: string;
        };
        /**
         * @description SNMPv3 USM authentication protocol. Variants are limited to the modern,
         *     secure set Scanopy supports; MD5 / SHA-2 variants beyond these are
         *     intentionally excluded. Serialized form (e.g. "Sha256") is the wire value
         *     stored in the credential and used as the frontend select option value.
         * @enum {string}
         */
        SnmpV3AuthProtocol: "Sha1" | "Sha256";
        /**
         * @description SNMPv3 USM privacy (encryption) protocol.
         * @enum {string}
         */
        SnmpV3PrivProtocol: "Aes128" | "Aes256";
        /**
         * @example {
         *       "cidr": "192.168.1.0/24",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Local area network",
         *       "first_discovery_id": null,
         *       "id": "550e8400-e29b-41d4-a716-446655440004",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "LAN",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "subnet_type": "Lan",
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Subnet: components["schemas"]["SubnetBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        SubnetBase: {
            cidr: string;
            description?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            subnet_type: components["schemas"]["SubnetType"];
            tags: string[];
            virtualization?: null | components["schemas"]["SubnetVirtualization"];
        };
        /**
         * @description Fields that subnets can be ordered/grouped by.
         * @enum {string}
         */
        SubnetOrderField: "created_at" | "name" | "cidr" | "subnet_type" | "updated_at" | "network_id" | "last_seen_at";
        /** @enum {string} */
        SubnetType: "Internet" | "Remote" | "Gateway" | "VpnTunnel" | "Dmz" | "Lan" | "WiFi" | "IoT" | "Guest" | "DockerBridge" | "PodmanBridge" | "MacVlan" | "IpVlan" | "Management" | "Storage" | "Loopback" | "Unknown";
        /**
         * @description Virtualization metadata for subnets that belong to a virtual infrastructure.
         *     Consistent with HostVirtualization and ServiceVirtualization patterns.
         *     Points to the service that provides the virtualization (e.g., Docker daemon).
         */
        SubnetVirtualization: (components["schemas"]["DockerSubnetVirtualization"] & {
            /** @enum {string} */
            type: "Docker";
        }) | (components["schemas"]["PodmanSubnetVirtualization"] & {
            /** @enum {string} */
            type: "Podman";
        });
        /**
         * @example {
         *       "color": "Green",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Production environment resources",
         *       "id": "550e8400-e29b-41d4-a716-44665544000a",
         *       "is_application": false,
         *       "lineage_id": null,
         *       "name": "production",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Tag: components["schemas"]["TagBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        TagBase: {
            color: components["schemas"]["Color"];
            description?: string | null;
            is_application?: boolean;
            name: string;
            /** Format: uuid */
            organization_id: string;
        };
        /**
         * @description Fields that tags can be ordered/grouped by.
         * @enum {string}
         */
        TagOrderField: "created_at" | "name" | "color" | "updated_at" | "is_application";
        /** @description Request to test reachability of a daemon URL. */
        TestReachabilityRequest: {
            /** @description If true, also perform an HTTP GET to {url}/health after the TCP check */
            check_health?: boolean;
            /** @description Full URL of the daemon (e.g. "https://daemon.example.com:60073") */
            url: string;
        };
        /** @description Response from a reachability test. */
        TestReachabilityResponse: {
            /** @description Error message if not reachable */
            error?: string | null;
            /** @description Health check result (only present when check_health was true) */
            health?: boolean | null;
            /** @description Whether the TCP connection succeeded */
            reachable: boolean;
        };
        Topology: components["schemas"]["TopologyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        TopologyBase: {
            /** Format: uuid */
            network_id: string;
            options: components["schemas"]["TopologyOptions"];
        };
        /**
         * @description Bundle of entities + the built graph that feed the topology render, export,
         *     and share pipelines.
         *
         *     Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
         *     for either the live view (`snapshot_id = None`) or a point-in-time snapshot
         *     (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
         *     from these entities + the network's grouping options
         *     (`build_all_view_graphs`) — they are not persisted. The frontend selects the
         *     active view's slice client-side.
         */
        TopologyData: {
            /**
             * @description Views whose data is present in this entity set (L3/Workloads always;
             *     L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
             *     tags are used). The topology tab restricts a snapshot's view picker to
             *     these — you can't set up SNMP or create app tags on a historical
             *     snapshot — while the live view shows all views with setup prompts.
             */
            available_views?: components["schemas"]["TopologyView"][];
            bindings: components["schemas"]["Binding"][];
            dependencies: components["schemas"]["Dependency"][];
            edges?: {
                [key: string]: components["schemas"]["Edge"][];
            };
            hosts: components["schemas"]["Host"][];
            interfaces: components["schemas"]["Interface"][];
            ip_addresses: components["schemas"]["IPAddress"][];
            /**
             * @description Per-view graph built on request from the entities above + grouping
             *     options. Keyed by view so switching the active perspective is a
             *     client-side slice selection.
             */
            nodes?: {
                [key: string]: components["schemas"]["Node"][];
            };
            ports: components["schemas"]["Port"][];
            services: components["schemas"]["Service"][];
            subnets: components["schemas"]["Subnet"][];
            tags: components["schemas"]["Tag"][];
            vlans: components["schemas"]["Vlan"][];
        };
        TopologyLocalOptions: {
            /** @default true */
            bundle_edges: boolean;
            /**
             * @default [
             *       "Hypervisor"
             *     ]
             */
            hide_edge_types: components["schemas"]["EdgeTypeDiscriminants"][];
            /** @default false */
            no_fade_edges: boolean;
            /** @default true */
            show_minimap: boolean;
            /**
             * @default {
             *       "hidden_host_tag_ids": [],
             *       "hidden_service_tag_ids": [],
             *       "hidden_subnet_tag_ids": []
             *     }
             */
            tag_filter: components["schemas"]["TopologyTagFilter"];
        };
        TopologyOptions: {
            local: components["schemas"]["TopologyLocalOptions"];
            request: components["schemas"]["TopologyRequestOptions"];
        };
        TopologyRequestOptions: {
            /**
             * @default {
             *       "Application": [
             *         {
             *           "id": "f6a64970-26dc-49ad-96c8-7763be711801",
             *           "rule": {
             *             "ByApplication": {
             *               "tag_ids": []
             *             }
             *           }
             *         }
             *       ],
             *       "L2Physical": [
             *         {
             *           "id": "8db5ea75-ab4c-4a80-95ab-5a5876db5e97",
             *           "rule": "ByHost"
             *         }
             *       ],
             *       "L3Logical": [
             *         {
             *           "id": "4ad63ccd-9c94-4bea-97e5-b18d6d9b5f7d",
             *           "rule": "BySubnet"
             *         },
             *         {
             *           "id": "669fb76d-0ed0-419c-8b67-d8417965a855",
             *           "rule": "MergeContainerBridges"
             *         }
             *       ],
             *       "Workloads": [
             *         {
             *           "id": "8db5ea75-ab4c-4a80-95ab-5a5876db5e97",
             *           "rule": "ByHost"
             *         }
             *       ]
             *     }
             */
            container_rules: {
                [key: string]: components["schemas"]["IdentifiedRule_ContainerRule"][];
            };
            /**
             * @default [
             *       {
             *         "id": "b61f1d54-78d9-4a42-99bf-9ebd3d132a81",
             *         "rule": "ByTrunkPort"
             *       },
             *       {
             *         "id": "fdaaa9b7-2a7c-46b5-97fb-6a72013b9e3e",
             *         "rule": "ByVLAN"
             *       },
             *       {
             *         "id": "b4f5d679-162f-45ed-94a1-de810d9ef51a",
             *         "rule": "ByPortOpStatus"
             *       },
             *       {
             *         "id": "11c27240-0383-49d1-9ec9-8f04ed2ec813",
             *         "rule": {
             *           "ByServiceCategory": {
             *             "categories": [
             *               "NetworkCore",
             *               "NetworkAccess",
             *               "RemoteAccess",
             *               "Workstation",
             *               "Mobile",
             *               "Printer",
             *               "OpenPorts"
             *             ],
             *             "is_infra_rule": true,
             *             "title": "Infrastructure"
             *           }
             *         }
             *       },
             *       {
             *         "id": "d96da2e3-8226-46d7-aff2-deb9b71f31dd",
             *         "rule": {
             *           "ByTag": {
             *             "tag_ids": [],
             *             "title": null
             *           }
             *         }
             *       },
             *       {
             *         "id": "6377cfeb-2d75-4349-a87c-26e12717302c",
             *         "rule": "ByHypervisor"
             *       },
             *       {
             *         "id": "4b025a32-75bd-4f8f-a389-25f2fc8a5be0",
             *         "rule": "ByContainerRuntime"
             *       },
             *       {
             *         "id": "bd688fed-55e5-42de-b362-cecec621b964",
             *         "rule": "ByStack"
             *       }
             *     ]
             */
            element_rules: components["schemas"]["IdentifiedRule_ElementRule"][];
            /**
             * @description Entity types hidden per view. Keyed by TopologyView, values are entity
             *     types (matching those declared as container/element/inline in the
             *     view's element_config). Hides every manifestation of the entity in
             *     that view — element nodes, container nodes, and inline rows on
             *     element cards. Supersedes the old `hide_ports` (L3-only, inline-only).
             * @default {}
             */
            hide_entities: {
                [key: string]: components["schemas"]["EntityDiscriminants"][];
            };
            /**
             * @description Generic per-(view, entity, filter) hide-set for metadata filters
             *     (Category, Virtualization, etc). Supersedes the old
             *     `hide_service_categories`; nested so JSON keys are strings all the
             *     way down.
             * @default {
             *       "Application": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "L2Physical": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "L3Logical": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "Workloads": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       }
             *     }
             */
            hide_metadata_values: {
                [key: string]: {
                    [key: string]: {
                        [key: string]: string[];
                    };
                };
            };
        };
        /** @description Filter settings for hiding entities by tag in topology visualization. */
        TopologyTagFilter: {
            /** @description Host tag IDs to hide (hosts with these tags will fade out) */
            hidden_host_tag_ids?: string[];
            /** @description Service tag IDs to hide (services with these tags will be hidden from nodes) */
            hidden_service_tag_ids?: string[];
            /** @description Subnet tag IDs to hide (subnets with these tags will fade out) */
            hidden_subnet_tag_ids?: string[];
        };
        /**
         * @description Which topology view is being rendered
         * @enum {string}
         */
        TopologyView: "L2Physical" | "L3Logical" | "Workloads" | "Application";
        /** @enum {string} */
        TransportProtocol: "Udp" | "Tcp";
        /**
         * @description Request type for updating a host with its children.
         *     Uses the same input types as CreateHostRequest.
         *     Server will sync children (create new, update existing, delete removed) only if provided.
         */
        UpdateHostRequest: {
            /**
             * @description Credential assignments for this host.
             *     If provided, replaces all existing credential assignments.
             */
            credential_assignments?: components["schemas"]["CredentialAssignment"][] | null;
            description?: string | null;
            /**
             * Format: date-time
             * @description Optional: expected updated_at timestamp for optimistic locking.
             */
            expected_updated_at?: string | null;
            hidden: boolean;
            hostname?: string | null;
            /** Format: uuid */
            id: string;
            /**
             * @description Interfaces to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing ip_addresses are preserved.
             */
            ip_addresses?: components["schemas"]["IPAddressInput"][] | null;
            name: string;
            /**
             * @description Ports to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing ports are preserved.
             */
            ports?: components["schemas"]["PortInput"][] | null;
            /**
             * @description Services to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing services are preserved.
             */
            services?: components["schemas"]["ServiceInput"][] | null;
            tags: string[];
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        UpdatePasswordRequest: {
            /**
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users adding their first password.
             */
            current_password?: string | null;
            /** @description New password to set */
            new_password: string;
        };
        /** @enum {string} */
        UseCase: "homelab" | "internal_it" | "msp" | "other";
        User: components["schemas"]["UserBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        UserApiKey: components["schemas"]["UserApiKeyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        UserApiKeyBase: {
            /** Format: date-time */
            expires_at?: string | null;
            is_enabled?: boolean;
            readonly key: string;
            /** Format: date-time */
            readonly last_used: string | null;
            name: string;
            /** @description Network IDs this key has access to (hydrated from junction table) */
            network_ids?: string[];
            /** Format: uuid */
            organization_id: string;
            permissions?: components["schemas"]["UserOrgPermissions"];
            tags: string[];
            /** Format: uuid */
            user_id: string;
        };
        /**
         * @description Response for user API key creation/rotation
         *     Contains the full API key record plus the plaintext key (shown only once)
         */
        UserApiKeyResponse: {
            api_key: components["schemas"]["UserApiKey"];
            /** @description The plaintext API key - only returned once during creation or rotation */
            key: string;
        };
        UserBase: {
            email: string;
            /** @description Per-user email preferences */
            email_settings?: components["schemas"]["EmailSettings"];
            /** @description Whether the user has verified their email address */
            email_verified?: boolean;
            /** @description Whether the user has a password set — computed from password_hash, never stored in DB */
            readonly has_password?: boolean;
            network_ids: string[];
            /** Format: date-time */
            oidc_linked_at?: string | null;
            oidc_provider?: string | null;
            /** Format: uuid */
            organization_id: string;
            permissions: components["schemas"]["UserOrgPermissions"];
            /** Format: date-time */
            readonly terms_accepted_at?: string | null;
        };
        /** @enum {string} */
        UserOrgPermissions: "Owner" | "Admin" | "Member" | "Viewer";
        /**
         * @description 2D unsigned coordinate. Used for node positions and sizes.
         *     Element node sizes are computed by the frontend (elkjs); the backend
         *     sets `Uxy::default()` for element nodes.
         */
        Uxy: {
            x: number;
            y: number;
        };
        VCenterVirtualization: {
            /** Format: uuid */
            service_id: string;
            vm_id?: string | null;
            vm_name?: string | null;
        };
        /** @description Request to verify email using token */
        VerifyEmailRequest: {
            token: string;
        };
        /**
         * @description Health status for daemon versions
         * @enum {string}
         */
        VersionHealthStatus: "Current" | "Outdated" | "Deprecated";
        /** @description Version information for API compatibility checking */
        VersionInfo: {
            /**
             * Format: int32
             * @description Current API version (integer, increments on breaking changes)
             */
            api_version: number;
            /** @description Minimum client version that can use this API (optional, for future use) */
            min_compatible_client?: string | null;
            /**
             * @description Server version (semver)
             * @example 0.12.10
             */
            server_version: string;
        };
        Vlan: components["schemas"]["VlanBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly first_discovery_id?: string | null;
            /** Format: uuid */
            readonly id: string;
            /** Format: uuid */
            readonly last_discovery_id?: string | null;
            /** Format: date-time */
            readonly last_seen_at?: string;
            /** Format: uuid */
            readonly lineage_id?: string | null;
            /** Format: date-time */
            readonly updated_at: string;
            /** Format: date-time */
            readonly valid_from?: string;
            /** Format: date-time */
            readonly valid_to?: string | null;
        };
        VlanBase: {
            description?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** Format: uuid */
            organization_id: string;
            source?: components["schemas"]["EntitySource"];
            /**
             * Format: int32
             * @description The 802.1Q VLAN number (1-4094)
             */
            vlan_number: number;
        };
        VlanDiscoveryItem: {
            name: string;
            /** Format: int32 */
            vlan_number: number;
        };
        /** @description Request body for daemon VLAN discovery upsert */
        VlanDiscoveryRequest: {
            /** Format: uuid */
            network_id: string;
            vlans: components["schemas"]["VlanDiscoveryItem"][];
        };
        /** @description Response for discovery upsert */
        VlanDiscoveryResponse: {
            /** @description Mapping of vlan_number → VLAN entity UUID */
            vlans: components["schemas"]["VlanDiscoveryResponseItem"][];
        };
        VlanDiscoveryResponseItem: {
            /** Format: uuid */
            id: string;
            /** Format: int32 */
            vlan_number: number;
        };
        /** @enum {string} */
        VlanOrderField: "created_at" | "name" | "vlan_number" | "updated_at";
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    check_email: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CheckEmailRequest"];
            };
        };
        responses: {
            /** @description Email is available */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Email already in use */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    forgot_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ForgotPasswordRequest"];
            };
        };
        responses: {
            /** @description Password reset email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginRequest"];
            };
        };
        responses: {
            /** @description Login successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid credentials */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Login forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Logout successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    get_current_user: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current user */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    unlink_oidc_account: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description OIDC provider slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OIDC account unlinked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Blocked in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Provider not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    onboarding_state: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Onboarding state */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_OnboardingStateResponse"];
                };
            };
        };
    };
    onboarding_step: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OnboardingStepRequest"];
            };
        };
        responses: {
            /** @description Step saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    register: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RegisterRequest"];
            };
        };
        responses: {
            /** @description User registered successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Registration disabled */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Email already exists */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    request_email_change: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RequestEmailChangeRequest"];
            };
        };
        responses: {
            /** @description Verification email sent to new address */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    resend_verification: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResendVerificationRequest"];
            };
        };
        responses: {
            /** @description Verification email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request or already verified */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Rate limited */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    reset_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResetPasswordRequest"];
            };
        };
        responses: {
            /** @description Password reset successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid or expired token */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    setup: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetupRequest"];
            };
        };
        responses: {
            /** @description Setup data stored */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SetupResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_password_auth: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdatePasswordRequest"];
            };
        };
        responses: {
            /** @description Password updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Blocked in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    verify_email: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VerifyEmailRequest"];
            };
        };
        responses: {
            /** @description Email verified successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid or expired token */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    cancel_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CancelSubscriptionRequest"];
            };
        };
        responses: {
            /** @description Cancellation initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_CancelSubscriptionResponse"];
                };
            };
            /** @description No active subscription or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    apply_discount_save_offer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discount applied */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Discount not configured or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    change_plan: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangePlanRequest"];
            };
        };
        responses: {
            /** @description Plan change initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Invalid plan or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    preview_plan_change: {
        parameters: {
            query: {
                /** @description Target plan (JSON) */
                plan: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plan change preview */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ChangePlanPreview"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_checkout_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateCheckoutRequest"];
            };
        };
        responses: {
            /** @description Checkout session URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Invalid plan or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    extend_trial: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Trial extended */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Ineligible or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    finalize_payment_method: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizePaymentMethodRequest"];
            };
        };
        responses: {
            /** @description Payment method finalized */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Billing not enabled or SetupIntent invalid */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    submit_enterprise_inquiry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EnterpriseInquiryRequest"];
            };
        };
        responses: {
            /** @description Inquiry submitted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request or Brevo not configured */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    pause_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PauseSubscriptionRequest"];
            };
        };
        responses: {
            /** @description Subscription paused */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Ineligible or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_payment_method_setup_intent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SetupIntent client secret */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SetupIntentResponse"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_billing_plans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of available billing plans */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_BillingPlan"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_portal_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Portal session URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    reactivate_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subscription reactivated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description No pending cancellation or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    resume_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subscription resumed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description No paused subscription or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_save_offer_coupon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Save-offer coupon terms, or null when not configured */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Option_SaveOfferCoupon"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    handle_webhook: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Webhook processed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid signature or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_public_config: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Public server configuration */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_PublicConfigResponse"];
                };
            };
        };
    };
    register_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonRegistrationRequest"];
            };
        };
        responses: {
            /** @description Daemon registered successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonRegistrationResponse"];
                };
            };
            /** @description Daemon registration disabled in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    receive_heartbeat: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonHeartbeatPayload"];
            };
        };
        responses: {
            /** @description Heartbeat received */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    receive_work_request: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonStatus"];
            };
        };
        responses: {
            /** @description Work request processed - returns (Option<Value>, bool) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    daemon_startup: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonStartupRequest"];
            };
        };
        responses: {
            /** @description Startup acknowledged */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ServerCapabilities"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_capabilities: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LegacyCapabilities"];
            };
        };
        responses: {
            /** @description Capabilities updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_stars: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description GitHub star count */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_u32"];
                };
            };
        };
    };
    list_daemon_api_keys: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Daemon API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["DaemonApiKey"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonApiKey"];
            };
        };
        responses: {
            /** @description Daemon API key created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKeyResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Insufficient permissions (member+ required) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_daemon_api_keys: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of daemon_api_key IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description daemon_api_keys deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description One or more API keys are in use by daemons */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_daemon_api_keys_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Daemon API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_daemon_api_key_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API Key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon API Key found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKey"];
                };
            };
            /** @description Daemon API Key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonApiKey"];
            };
        };
        responses: {
            /** @description Daemon API key updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKey"];
                };
            };
            /** @description Daemon API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description daemon_api_key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description daemon_api_key deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description daemon_api_key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key is in use by a daemon */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    rotate_key_handler: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon API key rotated, returns new key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Daemon API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_user_api_keys: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of user API keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_UserApiKey"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UserApiKey"];
            };
        };
        responses: {
            /** @description API key created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKeyResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Invalid permissions or network access */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_user_api_keys: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of API key IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description API keys deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_user_api_keys_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing User API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_user_api_key_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKey"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UserApiKey"];
            };
        };
        responses: {
            /** @description API key updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKey"];
                };
            };
            /** @description Not authorized to update this key */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    rotate_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key rotated, returns new key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Not authorized to rotate this key */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_bindings: {
        parameters: {
            query?: {
                /** @description Filter by service ID */
                service_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by port ID */
                port_id?: string | null;
                /** @description Filter by interface ID */
                ip_address_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Bindings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Binding"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_binding: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Binding"];
            };
        };
        responses: {
            /** @description Binding created (superseded bindings may be removed) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Referenced port or ip_address does not exist */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Conflict with existing binding type */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_bindings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Binding IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Bindings deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_bindings_csv: {
        parameters: {
            query?: {
                /** @description Filter by service ID */
                service_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by port ID */
                port_id?: string | null;
                /** @description Filter by interface ID */
                ip_address_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Bindings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_binding_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Binding found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Binding not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_binding: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Binding"];
            };
        };
        responses: {
            /** @description Binding updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Referenced port or ip_address does not exist */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Conflict with existing binding type */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_binding: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Binding deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Binding not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_credentials: {
        parameters: {
            query?: {
                /** @description Filter by credential type (e.g. "Snmp", "DockerProxy") */
                type?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Credential"];
                };
            };
        };
    };
    create_credential: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"];
            };
        };
        responses: {
            /** @description Credential created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_create_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"][];
            };
        };
        responses: {
            /** @description Credentials created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_Credential"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Credentials deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_credentials_csv: {
        parameters: {
            query?: {
                /** @description Filter by credential type (e.g. "Snmp", "DockerProxy") */
                type?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_by_id_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Credential found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
                };
            };
            /** @description Credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"];
            };
        };
        responses: {
            /** @description Credential updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Credential deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_daemons: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of daemons */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_DaemonResponse"];
                };
            };
        };
    };
    bulk_delete_daemons: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of daemon IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description daemons deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description daemon has active sessions */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    email_install_command: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EmailInstallCommandRequest"];
            };
        };
        responses: {
            /** @description Email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Email service not configured */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_daemons_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Daemons */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    provision_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProvisionDaemonRequest"];
            };
        };
        responses: {
            /** @description Daemon provisioned successfully */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ProvisionDaemonResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Daemon is live and already has a bound key */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    test_daemon_reachability: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TestReachabilityRequest"];
            };
        };
        responses: {
            /** @description Reachability test result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_TestReachabilityResponse"];
                };
            };
            /** @description Invalid URL */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_daemon_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Daemon"];
            };
        };
        responses: {
            /** @description daemon updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Daemon"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description daemon deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description daemon has active sessions */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_daemon_install_command: {
        parameters: {
            query: {
                /** @description `install` (with the api-key placeholder) or `reconfigure` (credential-free). */
                purpose: components["schemas"]["InstallCommandKind"];
                log_level?: string | null;
                log_file?: string | null;
                heartbeat_interval?: number | null;
                bind_address?: string | null;
                allow_self_signed_certs?: boolean | null;
                accept_invalid_scan_certs?: boolean | null;
                /** @description Comma-separated interface names. */
                interfaces?: string | null;
                /** @description Comma-separated credential/integration tokens (for the docker-compose env). */
                credential_refs?: string | null;
            };
            header?: never;
            path: {
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Install command */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_InstallArtifacts"];
                };
            };
            /** @description daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    retry_daemon_connection: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Connection retry initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_dashboard_summary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dashboard summary */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DashboardSummary"];
                };
            };
        };
    };
    get_all_dependencies: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of dependencies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Dependency"];
                };
            };
        };
    };
    create_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Dependency"];
            };
        };
        responses: {
            /** @description Dependency created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_dependencies: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Dependency IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Dependencies deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_dependencies_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Dependencies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_dependency_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dependency found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
                };
            };
            /** @description Dependency not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Dependency"];
            };
        };
        responses: {
            /** @description Dependency updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Dependency not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dependency deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Dependency not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_discoveries: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Discovery"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Discovery"];
            };
        };
        responses: {
            /** @description Discovery created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Can't create historical discovery */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_active_sessions: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active discovery sessions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_DiscoveryUpdatePayload"];
                };
            };
        };
    };
    bulk_delete_discoveries: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of discovery IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description discoveries deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description discovery has active session */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_discoveries_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    start_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Discovery session started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DiscoveryUpdatePayload"];
                };
            };
            /** @description Discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description A session is already running for this discovery */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_discovery_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discovery found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Discovery"];
            };
        };
        responses: {
            /** @description Discovery updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Can't update historical discovery */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description discovery deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description discovery has active session */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    cancel_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Session ID */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discovery session cancelled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    receive_discovery_update: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery session ID */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DiscoveryUpdatePayload"];
            };
        };
        responses: {
            /** @description Update received */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    get_all_hosts: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of hosts with their children */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_HostResponse"];
                };
            };
        };
    };
    create_host: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateHostRequest"];
            };
        };
        responses: {
            /** @description Host created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: network not found, subnet mismatch, or invalid tags */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description No access to the specified network */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_hosts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of host IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Hosts deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description One or more hosts has an associated daemon - delete daemons first */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_host_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DiscoveryHostRequest"];
            };
        };
        responses: {
            /** @description Host discovered/updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Daemon cannot create hosts on other networks */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_hosts_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Hosts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    export_hosts_zip: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ZIP file containing CSVs */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/zip": unknown;
                };
            };
        };
    };
    consolidate_hosts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Destination host ID - will receive all children */
                destination_host: string;
                /** @description Host to merge into destination - will be deleted */
                other_host: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Hosts consolidated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: same host, has daemon, or different networks */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description One or both hosts not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_host_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Host found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateHostRequest"];
            };
        };
        responses: {
            /** @description Host updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: invalid tags */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Host deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Host has associated daemon */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_interfaces: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Interfaces */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Interface"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_if_entry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description If entry created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Network mismatch or duplicate if_index */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_interfaces: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Interface IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Interfaces deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_interfaces_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Interfaces */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_interface_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Interface ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Interface found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Interface not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_if_entry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description If entry ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description If entry updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description If entry not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_interface: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Interface ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Interface deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Interface not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_invites: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active invites */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_Invite"];
                };
            };
        };
    };
    create_invite: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateInviteRequest"];
            };
        };
        responses: {
            /** @description Invite created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Invite"];
                };
            };
            /** @description Cannot create invite with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_invite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Invite ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invite details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Invite"];
                };
            };
            /** @description Invalid or expired invite */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    revoke_invite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Invite ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invite revoked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid invite */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Cannot revoke this invite */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_ip_addresses: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by subnet ID */
                subnet_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of IP Addresses */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["IPAddress"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IPAddress"];
            };
        };
        responses: {
            /** @description IP address created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_ip_addresses: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description IP addresses deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description No IDs provided */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_ip_addresses_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by subnet ID */
                subnet_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing IP Addresses */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_ip_address_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP Address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IP Address found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
                };
            };
            /** @description IP Address not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IPAddress"];
            };
        };
        responses: {
            /** @description IP address updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description IP address not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IP address deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description IP address not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_networks: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of networks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: (components["schemas"]["NetworkBase"] & {
                            /** Format: date-time */
                            readonly created_at: string;
                            /**
                             * Format: int64
                             * @description `stale_after_hours` with the server's default already applied.
                             *
                             *     Computed, never stored (excluded from `to_params`). Published so the
                             *     frontend derives staleness from the *same* number the digest uses rather
                             *     than re-declaring the default in TypeScript, where the two could drift
                             *     and a host could read stale in the app but current in the digest email.
                             */
                            readonly effective_stale_after_hours?: number;
                            /** Format: uuid */
                            readonly id: string;
                            /** Format: date-time */
                            readonly updated_at: string;
                        })[];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_network: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Network"];
            };
        };
        responses: {
            /** @description Network created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
        };
    };
    bulk_delete_networks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of network IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Networks deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_networks_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Networks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_by_id_network: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Network found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_network: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Network"];
            };
        };
        responses: {
            /** @description Network updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_network: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Network deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_organization: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Organization"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    daemon_prompt_response: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonPromptResponseRequest"];
            };
        };
        responses: {
            /** @description Response recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    update_profile: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProfileUpdateRequest"];
            };
        };
        responses: {
            /** @description Profile updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    submit_referral_source: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReferralSourceRequest"];
            };
        };
        responses: {
            /** @description Referral source recorded */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    update_org_name: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Organization updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Organization"];
                };
            };
            /** @description Only owners can update organization */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_organization: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot delete another organization */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    populate_demo_data: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Demo data populated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Only available for demo organizations */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    reset: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization reset */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot reset another organization */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_ports: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Ports */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Port"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_port: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Port"];
            };
        };
        responses: {
            /** @description Port created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Network mismatch or duplicate port */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_ports: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Port IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Ports deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_ports_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Ports */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_port_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Port found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_port: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Port"];
            };
        };
        responses: {
            /** @description Port updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_port: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Port deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_services: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns services that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Exclude services belonging to these categories. */
                exclude_categories?: components["schemas"]["ServiceCategory"][] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only services discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the service's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of services */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Service"];
                };
            };
        };
    };
    create_service: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateServiceRequest"];
            };
        };
        responses: {
            /** @description Service created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Validation error: host network mismatch, cross-host binding, or binding conflict */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_services: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Service IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Services deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_services_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns services that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Exclude services belonging to these categories. */
                exclude_categories?: components["schemas"]["ServiceCategory"][] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only services discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the service's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Services */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_service_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Service found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_service: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Service"];
            };
        };
        responses: {
            /** @description Service updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Validation error: host network mismatch, cross-host binding, or binding conflict */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_service: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Service deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_shares: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by topology ID */
                topology_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Shares */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Share"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_share: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUpdateShareRequest"];
            };
        };
        responses: {
            /** @description Share created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_shares: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Share IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Shares deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_shares_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by topology ID */
                topology_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Shares */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_public_share_metadata: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share metadata */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_PublicShareMetadata"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    verify_share_password: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Password verified; access token issued */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ShareAccessTokenResponse"];
                };
            };
            /** @description Invalid password */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_share_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_share: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUpdateShareRequest"];
            };
        };
        responses: {
            /** @description Share updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_share: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_snapshots: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Snapshots */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Snapshot"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_snapshot: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSnapshotRequest"];
            };
        };
        responses: {
            /** @description Snapshot created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Snapshot"];
                };
            };
            /** @description Snapshots not available on plan */
            402: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network is busy with discovery; retry shortly */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_snapshot_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Snapshot ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Snapshot found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Snapshot"];
                };
            };
            /** @description Snapshot not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_snapshot: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Snapshot ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Snapshot deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Snapshot not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_subnets: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only subnets discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the subnet's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of subnets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Subnet"];
                };
            };
        };
    };
    create_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Subnet"];
            };
        };
        responses: {
            /** @description Subnet created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_subnets: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Subnet IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Subnets deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_subnets_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only subnets discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the subnet's own network's window.
                 */
                stale?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Subnets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_subnet_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subnet found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Subnet"];
            };
        };
        responses: {
            /** @description Subnet updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description CIDR change would orphan existing ip_addresses */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subnet deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_tags: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["TagOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["TagOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of tags */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Tag"];
                };
            };
        };
    };
    create_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Tag"];
            };
        };
        responses: {
            /** @description Tag created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Validation error: name empty or too long */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag name already exists in this organization */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    set_entity_tags: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetTagsRequest"];
            };
        };
        responses: {
            /** @description Tags set successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid entity type or tag */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_add_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BulkTagRequest"];
            };
        };
        responses: {
            /** @description Tag added successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkTagResponse"];
                };
            };
            /** @description Invalid entity type or tag */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_remove_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BulkTagRequest"];
            };
        };
        responses: {
            /** @description Tag removed successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkTagResponse"];
                };
            };
            /** @description Invalid entity type */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_tags: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Tag IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Tags deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_tags_csv: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["TagOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["TagOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Tags */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_tag_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tag found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_tag: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Tag"];
            };
        };
        responses: {
            /** @description Tag updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_tag: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tag deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_topologies: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of topologies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Topology"];
                };
            };
        };
    };
    get_topology_data: {
        parameters: {
            query: {
                /** @description Network to read entities for. Required. */
                network_id: string;
                /**
                 * @description When set, returns the entity set as it was when this snapshot was taken.
                 *     When omitted, returns live entities.
                 */
                snapshot_id?: string | null;
                /**
                 * @description When `true`, records the `FirstTopologyRebuild` onboarding milestone (the user has
                 *     viewed their topology). Only the frontend's explicit on-tab view sets this — the
                 *     background topology-data query never does — so the milestone never fires from other
                 *     tabs. One-time per org (guarded below + subscriber dedup).
                 */
                mark_viewed?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology entity bundle */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_TopologyData"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Snapshot not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_topologies_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Topologies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_topology_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_topology: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Topology"];
            };
        };
        responses: {
            /** @description Topology updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_confluence: {
        parameters: {
            query?: {
                /** @description View to export. Defaults to the default view when omitted. */
                view?: components["schemas"]["TopologyView"];
            };
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Confluence wiki markup export */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": unknown;
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_mermaid: {
        parameters: {
            query?: {
                /** @description View to export. Defaults to the default view when omitted. */
                view?: components["schemas"]["TopologyView"];
            };
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Mermaid flowchart export */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": unknown;
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_users: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of users */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_User"];
                };
            };
        };
    };
    bulk_delete_users: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of user IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Users deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description Cannot delete users with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_users_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Users */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_user_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description User found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["User"];
            };
        };
        responses: {
            /** @description User updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Cannot update another user's record */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description User deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot delete user with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Cannot delete the only owner */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    admin_update_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["User"];
            };
        };
        responses: {
            /** @description User updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Cannot update user with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_vlans: {
        parameters: {
            query?: {
                group_by?: null | components["schemas"]["VlanOrderField"];
                order_by?: null | components["schemas"]["VlanOrderField"];
                order_direction?: null | components["schemas"]["OrderDirection"];
                limit?: number | null;
                offset?: number | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of VLANs */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Vlan"];
                };
            };
        };
    };
    create_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Vlan"];
            };
        };
        responses: {
            /** @description VLAN created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description VLAN number already exists in this network */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_vlans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Vlan IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Vlans deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    discovery_upsert_vlans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VlanDiscoveryRequest"];
            };
        };
        responses: {
            /** @description VLANs upserted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_VlanDiscoveryResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_vlans_csv: {
        parameters: {
            query?: {
                group_by?: null | components["schemas"]["VlanOrderField"];
                order_by?: null | components["schemas"]["VlanOrderField"];
                order_direction?: null | components["schemas"]["OrderDirection"];
                limit?: number | null;
                offset?: number | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Vlans */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_vlan_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Vlan found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
                };
            };
            /** @description Vlan not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Vlan"];
            };
        };
        responses: {
            /** @description Vlan updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
                };
            };
            /** @description Vlan not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Vlan deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Vlan not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_version: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Version information */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_VersionInfo"];
                };
            };
        };
    };
}
