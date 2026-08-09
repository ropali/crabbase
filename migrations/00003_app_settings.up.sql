-- ============================================================
-- App Settings Table
-- ============================================================

CREATE TABLE IF NOT EXISTS _settings (
    id      TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::text,
    name    VARCHAR(60) NOT NULL,
    value   TEXT DEFAULT NULL,
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================
-- System Collection Registration in _collections
-- ============================================================

INSERT INTO _collections (id, system, type, name, fields, options)
VALUES (
    'r' || substring(md5(random()::text) from 1 for 14),
    1,
    'base',
    '_settings',
    '[
        {"name": "id", "type": "text", "required": true},
        {"name": "name", "type": "text", "required": true},
        {"name": "value", "type": "json", "required": false},
        {"name": "created", "type": "autodate"},
        {"name": "updated", "type": "autodate"}
    ]'::jsonb,
    '{}'::jsonb
)
ON CONFLICT (name) DO NOTHING;

-- ============================================================
-- Default Settings
-- ============================================================

INSERT INTO _settings (name, value)
VALUES (
    'email_templates',
    $json${
  "passwordReset": {
    "subject": "Password Reset Link",
    "body_html": "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Password Reset</title></head><body style=\"margin:0;padding:0;background-color:#f4f4f4;font-family:Arial,Helvetica,sans-serif;color:#333333;\"><table role=\"presentation\" width=\"100%\" cellspacing=\"0\" cellpadding=\"0\" style=\"background-color:#f4f4f4;padding:32px 0;\"><tr><td align=\"center\"><table role=\"presentation\" width=\"600\" cellspacing=\"0\" cellpadding=\"0\" style=\"background:#ffffff;border-radius:8px;padding:40px;max-width:600px;\"><tr><td><h2 style=\"margin:0 0 24px;font-size:24px;color:#222222;\">Reset Your Password</h2><p style=\"margin:0 0 16px;font-size:16px;line-height:1.6;\">Hello,</p><p style=\"margin:0 0 16px;font-size:16px;line-height:1.6;\">We received a request to reset the password for your account. Click the button below to choose a new password.</p><p style=\"margin:32px 0;text-align:center;\"><a href=\"{{reset_link}}\" style=\"display:inline-block;padding:14px 28px;background-color:#2563eb;color:#ffffff;text-decoration:none;border-radius:6px;font-size:16px;font-weight:bold;\">Reset Password</a></p><p style=\"margin:0 0 16px;font-size:14px;line-height:1.6;\">If the button above does not work, copy and paste the following link into your browser:</p><p style=\"margin:0 0 24px;font-size:14px;word-break:break-all;\"><a href=\"{{reset_link}}\" style=\"color:#2563eb;\">{{reset_link}}</a></p><p style=\"margin:0 0 16px;font-size:14px;line-height:1.6;\">If you did not request a password reset, you can safely ignore this email. Your password will remain unchanged.</p><p style=\"margin:24px 0 0;font-size:14px;line-height:1.6;\">Thanks,<br>The Team</p></td></tr></table></td></tr></table></body></html>",
    "body_text": "Hello,\n\nWe received a request to reset the password for your account.\n\nTo choose a new password, open the following link:\n\n{{reset_link}}\n\nIf you did not request a password reset, you can safely ignore this email. Your password will remain unchanged.\n\nThanks,\nThe Team"
  },
  "userRegistration": {
    "subject": "Welcome! Verify Your Email",
    "body_html": "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\"><title>Verify Your Email</title></head><body style=\"margin:0;padding:0;background-color:#f4f4f4;font-family:Arial,Helvetica,sans-serif;color:#333333;\"><table role=\"presentation\" width=\"100%\" cellspacing=\"0\" cellpadding=\"0\" style=\"background-color:#f4f4f4;padding:32px 0;\"><tr><td align=\"center\"><table role=\"presentation\" width=\"600\" cellspacing=\"0\" cellpadding=\"0\" style=\"background:#ffffff;border-radius:8px;padding:40px;max-width:600px;\"><tr><td><h2 style=\"margin:0 0 24px;font-size:24px;color:#222222;\">Welcome!</h2><p style=\"margin:0 0 16px;font-size:16px;line-height:1.6;\">Hello,</p><p style=\"margin:0 0 16px;font-size:16px;line-height:1.6;\">Thank you for creating an account. To complete your registration, please verify your email address by clicking the button below.</p><p style=\"margin:32px 0;text-align:center;\"><a href=\"{{verification_link}}\" style=\"display:inline-block;padding:14px 28px;background-color:#16a34a;color:#ffffff;text-decoration:none;border-radius:6px;font-size:16px;font-weight:bold;\">Verify Email</a></p><p style=\"margin:0 0 16px;font-size:14px;line-height:1.6;\">If the button above does not work, copy and paste the following link into your browser:</p><p style=\"margin:0 0 24px;font-size:14px;word-break:break-all;\"><a href=\"{{verification_link}}\" style=\"color:#16a34a;\">{{verification_link}}</a></p><p style=\"margin:0 0 16px;font-size:14px;line-height:1.6;\">If you did not create this account, you can safely ignore this email.</p><p style=\"margin:24px 0 0;font-size:14px;line-height:1.6;\">Welcome aboard,<br>The Team</p></td></tr></table></td></tr></table></body></html>",
    "body_text": "Hello,\n\nThank you for creating an account.\n\nPlease verify your email address by opening the following link:\n\n{{verification_link}}\n\nIf you did not create this account, you can safely ignore this email.\n\nWelcome aboard,\nThe Team"
  }
}$json$
)
ON CONFLICT (id) DO NOTHING;
