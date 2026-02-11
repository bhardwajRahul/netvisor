// Email template constants

pub const EMAIL_HEADER: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Scanopy</title>
</head>
<body style="margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; background-color: #f5f5f5;">
    <table role="presentation" style="width: 100%; border-collapse: collapse; background-color: #f5f5f5;">
        <tr>
            <td align="center" style="padding: 40px 20px;">
                <table role="presentation" style="max-width: 600px; width: 100%; border-collapse: collapse; background-color: #ffffff; border-radius: 8px; box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);">
                    <!-- Header with Logo -->
                    <tr>
                        <td align="center" style="padding: 40px 40px 30px 40px;">
                            <img src="https://cdn.jsdelivr.net/gh/scanopy/scanopy@main/media/logo.png" alt="Scanopy" style="width: 80px; height: 80px; display: block;">
                        </td>
                    </tr>
"#;

pub const EMAIL_FOOTER: &str = r#"                    <!-- Footer -->
                    <tr>
                        <td align="center" style="padding: 30px 40px 20px 40px; background-color: #f9fafb; border-radius: 0 0 8px 8px;">
                            <!-- Social Links -->
                            <table role="presentation" style="margin: 0 auto 20px auto; border-collapse: collapse;">
                                <tr>
                                    <td style="padding: 0 10px;">
                                        <a href="https://discord.com/invite/b7ffQr8AcZ" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/discord.png" alt="Discord" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                    <td style="padding: 0 10px;">
                                        <a href="https://github.com/scanopy/scanopy" style="display: inline-block;">
                                            <img src="https://cdn.jsdelivr.net/gh/selfhst/icons@master/png/github.png" alt="GitHub" style="width: 24px; height: 24px; display: block;">
                                        </a>
                                    </td>
                                </tr>
                            </table>
                            
                            <p style="margin: 0; font-size: 12px; line-height: 18px; color: #9ca3af;">© 2025 Scanopy. All rights reserved.</p>
                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
"#;

pub const PASSWORD_RESET_TITLE: &str = "Scanopy Password Reset";

pub const PASSWORD_RESET_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Reset Your Password</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">We received a request to reset your password for your Scanopy account. Click the button below to create a new password:</p>
                        </td>
                    </tr>
                    
                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{reset_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Reset Password</a>
                        </td>
                    </tr>
                    
                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{reset_url}</p>
                        </td>
                    </tr>
                    
                    <!-- Security Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This password reset link will expire in 24 hours. If you didn't request a password reset, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;

pub const INVITE_LINK_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">You've Been Invited to Scanopy</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">{inviter_name} has invited you to join their Scanopy instance to visualize and explore their network infrastructure.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Click the button below to accept the invitation and create your account:</p>
                        </td>
                    </tr>
                    
                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{invite_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Accept Invitation</a>
                        </td>
                    </tr>
                    
                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{invite_url}</p>
                        </td>
                    </tr>
                    
                    <!-- Expiration Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This invitation link will expire in 7 days. If you didn't expect this invitation, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;

// ============================================================================
// Billing Templates
// ============================================================================

pub const TRIAL_STARTED_TITLE: &str = "Welcome to Scanopy - Your Trial Has Started";

pub const TRIAL_STARTED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Welcome to Scanopy {plan_name}!</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your trial of the {plan_name} plan has started. You have full access to all features for the next {trial_days} days.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">No credit card is required during the trial. Add a payment method anytime from your Settings page to continue after the trial ends.</p>
                        </td>
                    </tr>
"#;

pub const TRIAL_ENDING_TITLE: &str = "Your Scanopy Trial Ends in 3 Days";

pub const TRIAL_ENDING_BODY_NO_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} trial ends in 3 days. To keep all your features and data, add a payment method before the trial expires.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">If no payment method is added, your account will be downgraded to the Free plan, which includes up to 25 hosts with manual discovery only.</p>
                        </td>
                    </tr>
"#;

pub const TRIAL_ENDING_BODY_HAS_PAYMENT: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Ends Soon</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} trial ends in 3 days. The payment method you've added will be automatically billed at the end of the trial period.</p>
                        </td>
                    </tr>
"#;

pub const TRIAL_EXPIRED_TITLE: &str = "Your Scanopy Trial Has Ended";

pub const TRIAL_EXPIRED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Your Trial Has Ended</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your {plan_name} trial has ended and your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can still use Scanopy with up to 25 hosts and manual discovery. Upgrade anytime to restore scheduled discovery, DaemonPoll mode, and higher limits.</p>
                        </td>
                    </tr>
"#;

pub const PLAN_CHANGED_TITLE: &str = "Your Scanopy Plan Has Changed";

pub const PLAN_CHANGED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Plan Updated</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy plan has been changed to {plan_name}. The change takes effect immediately.</p>
                        </td>
                    </tr>
"#;

pub const SUBSCRIPTION_CANCELLED_TITLE: &str = "Your Scanopy Subscription Has Been Cancelled";

pub const SUBSCRIPTION_CANCELLED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Subscription Cancelled</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Your Scanopy subscription has been cancelled. Your account has been moved to the Free plan.</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">You can continue using Scanopy with up to 25 hosts and manual discovery. Resubscribe anytime from your Settings page.</p>
                        </td>
                    </tr>
"#;

pub const PAYMENT_METHOD_ADDED_TITLE: &str = "Payment Method Added - Scanopy";

pub const PAYMENT_METHOD_ADDED_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Payment Method Added</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">A payment method has been added to your Scanopy account. Your subscription will continue automatically when the trial ends.</p>
                        </td>
                    </tr>
"#;

// ============================================================================
// Auth Templates
// ============================================================================

pub const EMAIL_VERIFICATION_TITLE: &str = "Verify Your Email - Scanopy";

pub const EMAIL_VERIFICATION_BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Verify Your Email</h1>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Hi there,</p>
                            <p style="margin: 0 0 20px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Thanks for signing up for Scanopy! Please verify your email address by clicking the button below:</p>
                        </td>
                    </tr>

                    <!-- CTA Button -->
                    <tr>
                        <td align="center" style="padding: 0 40px 30px 40px;">
                            <a href="{verify_url}" style="display: inline-block; padding: 14px 40px; background-color: #2563eb; color: #ffffff; text-decoration: none; border-radius: 6px; font-size: 16px; font-weight: 500;">Verify Email</a>
                        </td>
                    </tr>

                    <!-- Alternative Link -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <p style="margin: 0 0 10px 0; font-size: 14px; line-height: 20px; color: #6b7280;">If the button doesn't work, copy and paste this link into your browser:</p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #2563eb; word-break: break-all;">{verify_url}</p>
                        </td>
                    </tr>

                    <!-- Expiration Notice -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 14px; line-height: 20px; color: #6b7280;">This verification link will expire in 24 hours. If you didn't create a Scanopy account, you can safely ignore this email.</p>
                        </td>
                    </tr>
"#;
