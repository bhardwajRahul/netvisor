> **First:** Read `CLAUDE.md` (project instructions) — you are a **worker**. Start in **plan mode** and propose your implementation before coding.

# Task: Add Email Verification to User Registration

## Objective

Implement email verification step during user registration. New users must verify their email address before their account becomes active.

## Background

The codebase already has email infrastructure:
- `backend/src/server/email/` - Email service with Plunk and SMTP providers
- `EmailProvider` trait with `send_password_reset()` and `send_invite()` methods
- HTML email templates in `templates.rs`

## Requirements

### Backend

1. **Database changes**
   - Add `email_verified: bool` column to users table (default false)
   - Add `email_verification_token: Option<String>` column
   - Add `email_verification_expires: Option<DateTime>` column

2. **Email verification endpoint**
   - `POST /api/auth/verify-email` - accepts token, marks user as verified
   - `POST /api/auth/resend-verification` - generates new token, sends email

3. **Registration flow changes**
   - On registration, generate verification token
   - Send verification email with link
   - User cannot login until verified (or limited access)

4. **EmailProvider extension**
   - Add `send_verification_email()` method to trait
   - Add email template for verification

5. **Token handling**
   - Secure random token generation
   - Expiration time (suggest 24 hours)
   - Single-use tokens

### Frontend

1. **Registration feedback**
   - After registration, show "check your email" message
   - Provide "resend verification" option

2. **Verification page**
   - `/verify-email?token=xxx` route
   - Show success/error state
   - Redirect to login on success

3. **Login handling**
   - Show appropriate error if email not verified
   - Offer to resend verification email

## Acceptance Criteria

- [ ] New users receive verification email on registration
- [ ] Clicking verification link activates account
- [ ] Unverified users cannot login (or have limited access)
- [ ] Verification tokens expire after 24 hours
- [ ] Users can request new verification email
- [ ] Existing users unaffected (migration sets verified=true)

## Files Likely Involved

### Backend
- `backend/migrations/` - New migration for columns
- `backend/src/server/email/traits.rs` - New method
- `backend/src/server/email/templates.rs` - New template
- `backend/src/server/auth/handlers.rs` - New endpoints
- `backend/src/server/auth/service.rs` - Verification logic
- `backend/src/server/users/impl/base.rs` - User model changes

### Frontend
- `ui/src/routes/` - New verify-email route
- `ui/src/lib/features/auth/` - Registration/login updates
- `ui/src/lib/api/` - New API calls

## Notes

- Follow existing email patterns exactly
- Verification link format: `{base_url}/verify-email?token={token}`
- Consider rate limiting resend endpoint

---

## Work Summary

### What was implemented

Email verification for user registration with the following features:
- New users must verify email before logging in
- OIDC users are auto-verified (identity provider already verifies)
- Self-hosted instances without email service auto-verify users on registration
- Password reset tokens migrated from in-memory HashMap to database for persistence across restarts and multi-instance deployments
- 60-second rate limiting on verification email resend

### Files Changed

**Backend:**
- `backend/migrations/20260110000000_email_verification.sql` - New migration adding 5 columns (email_verified, email_verification_token, email_verification_expires, password_reset_token, password_reset_expires) with indexes
- `backend/src/server/users/impl/base.rs` - Added new fields to UserBase struct, updated new_password/new_oidc constructors
- `backend/src/server/email/templates.rs` - Added EMAIL_VERIFICATION_TITLE and EMAIL_VERIFICATION_BODY constants
- `backend/src/server/email/traits.rs` - Added build_verification_email() and send_verification_email() to EmailProvider trait
- `backend/src/server/email/plunk.rs` - Implemented send_verification_email()
- `backend/src/server/email/smtp.rs` - Implemented send_verification_email()
- `backend/src/server/auth/service.rs` - Major changes: removed in-memory password_reset_tokens, added verification methods, modified register() and try_login()
- `backend/src/server/auth/handlers.rs` - Added verify_email and resend_verification routes and handlers
- `backend/src/server/auth/impl/api.rs` - Added VerifyEmailRequest and ResendVerificationRequest types
- `backend/src/server/shared/services/factory.rs` - Updated AuthService::new() to pass public_url
- `backend/src/server/shared/types/examples.rs` - Added new fields to example User

**Frontend:**
- `ui/src/lib/features/auth/types/base.ts` - Exported new request types
- `ui/src/lib/features/auth/queries.ts` - Added useVerifyEmailMutation() and useResendVerificationMutation()
- `ui/src/routes/verify-email/+page.svelte` - New verification page with multiple states
- `ui/src/lib/shared/components/layout/AppShell.svelte` - Added /verify-email to public routes
- `ui/src/routes/onboarding/+page.svelte` - Redirect to verify-email when user.email_verified is false
- `ui/src/routes/login/+page.svelte` - Handle EMAIL_NOT_VERIFIED error by redirecting to verify-email

### Endpoints Added

| Endpoint | Permission | Tenant Isolation |
|----------|------------|------------------|
| `POST /api/auth/verify-email` | Public (no auth) | Token-based validation, user lookup by token |
| `POST /api/auth/resend-verification` | Public (no auth) | Email-based lookup, rate limited |

These endpoints are public (like login/register) as they're used before authentication. Token validation ensures users can only verify their own accounts.

### Deviations from Original Task

1. **Added password reset token migration** - Per user request, migrated password reset tokens from in-memory storage to database for persistence across server restarts and multi-instance deployments
2. **Self-hosted fallback** - Added auto-verification for self-hosted instances without email service configured, ensuring they can still use the system
3. **Auto-login after verification** - Users are automatically logged in after successful verification (better UX than requiring re-login)

### Token Expiration

- Email verification: 24 hours
- Password reset: 1 hour (unchanged from original behavior)

### Acceptance Criteria Status

- [x] New users receive verification email on registration
- [x] Clicking verification link activates account
- [x] Unverified users cannot login (blocked with specific error)
- [x] Verification tokens expire after 24 hours
- [x] Users can request new verification email (with 60s rate limit)
- [x] Existing users unaffected (migration sets verified=true)
