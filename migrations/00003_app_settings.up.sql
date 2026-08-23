-- ============================================================
-- App Settings Table
-- ============================================================

CREATE TABLE IF NOT EXISTS _settings (
    id      TEXT PRIMARY KEY NOT NULL DEFAULT gen_random_uuid()::text,
    name    VARCHAR(60) UNIQUE NOT NULL,
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
        {"name": "name", "type": "text", "required": true},
        {"name": "value", "type": "json", "required": false}
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
    "subject": "Your password reset OTP",
    "bodyHtml": "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\"><title>Reset your password</title><style>body{margin:0;padding:0;background-color:#fff8f6;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#261816}.wrapper{max-width:560px;margin:40px auto;padding:0 16px 40px}.card{background:#ffffff;border:1px solid #e2bfb8;border-radius:16px;overflow:hidden}.header{background-color:#ab2815;padding:28px 40px;text-align:center}.header h1{margin:0;font-size:20px;font-weight:700;color:#ffffff;letter-spacing:-0.3px}.body{padding:36px 40px}.body p{margin:0 0 16px;font-size:14px;line-height:22px;color:#261816}.body p.muted{color:#5a413c;font-size:13px}.pill{display:inline-block;background:#ffe9e5;border:1px solid #e2bfb8;color:#ab2815;font-size:12px;font-weight:600;padding:4px 12px;border-radius:999px;margin-bottom:20px}.otp-box{text-align:center;margin:28px 0}.otp-code{display:inline-block;background:#fff0ed;border:2px dashed #ab2815;border-radius:12px;padding:18px 40px;font-size:36px;font-weight:800;letter-spacing:0.25em;color:#ab2815;font-family:monospace}.otp-hint{margin-top:10px;font-size:12px;color:#8e706b}.cta{text-align:center;margin:24px 0}.cta a{display:inline-block;background-color:#ab2815;color:#ffffff;font-size:14px;font-weight:700;text-decoration:none;padding:13px 32px;border-radius:10px;letter-spacing:0.2px}.divider{border:none;border-top:1px solid #e2bfb8;margin:24px 0}.footer{text-align:center;padding:16px 40px 28px;font-size:12px;color:#8e706b}.footer a{color:#ab2815;text-decoration:none}</style></head><body><div class=\"wrapper\"><div class=\"card\"><div class=\"header\"><h1>Reset your password</h1></div><div class=\"body\"><span class=\"pill\">Password Reset</span><p>Hi <strong>{{name}}</strong>,</p><p>We received a request to reset the password for your account associated with <strong>{{email}}</strong>. Use the OTP below to reset your password.</p><div class=\"otp-box\"><div class=\"otp-code\">{{otp}}</div><p class=\"otp-hint\">&#9200; This OTP is valid for <strong>5 minutes only</strong>.</p></div><div class=\"cta\"><a href=\"{{link}}/reset-password?email={{email}}\">Go to Reset Password Page</a></div><hr class=\"divider\"><p class=\"muted\">If you did not request a password reset, no action is needed &mdash; your password will remain unchanged.</p></div><div class=\"footer\"><p>You received this email because an action was performed on your account.<br>If you did not request this, you can safely ignore this email.</p></div></div></div></body></html>",
    "bodyText": "Hi {{name}},\n\nWe received a request to reset the password for your account ({{email}}).\n\nYour One-Time Password (OTP) is:\n\n  {{otp}}\n\nThis OTP is valid for 5 minutes only.\n\nGo to the reset password page and enter your OTP along with your new password:\n\n  {{link}}/reset-password\n\nIf you did not request this, ignore this email — your password will remain unchanged."
  },
  "userRegistration": {
    "subject": "Welcome!",
    "bodyHtml": "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"UTF-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1.0\"><title>Welcome!</title><style>body{margin:0;padding:0;background-color:#fff8f6;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;color:#261816}.wrapper{max-width:560px;margin:40px auto;padding:0 16px 40px}.card{background:#ffffff;border:1px solid #e2bfb8;border-radius:16px;overflow:hidden}.header{background-color:#ab2815;padding:28px 40px;text-align:center}.header h1{margin:0;font-size:20px;font-weight:700;color:#ffffff;letter-spacing:-0.3px}.body{padding:36px 40px}.body p{margin:0 0 16px;font-size:14px;line-height:22px;color:#261816}.body p.muted{color:#5a413c;font-size:13px}.pill{display:inline-block;background:#ffe9e5;border:1px solid #e2bfb8;color:#ab2815;font-size:12px;font-weight:600;padding:4px 12px;border-radius:999px;margin-bottom:20px}.cta{text-align:center;margin:28px 0}.cta a{display:inline-block;background-color:#ab2815;color:#ffffff;font-size:14px;font-weight:700;text-decoration:none;padding:13px 32px;border-radius:10px;letter-spacing:0.2px}.divider{border:none;border-top:1px solid #e2bfb8;margin:24px 0}.footer{text-align:center;padding:16px 40px 28px;font-size:12px;color:#8e706b}.footer a{color:#ab2815;text-decoration:none}</style></head><body><div class=\"wrapper\"><div class=\"card\"><div class=\"header\"><h1>Welcome!</h1></div><div class=\"body\"><span class=\"pill\">New account</span><p>Hi <strong>{{name}}</strong>,</p><p>Your account has been successfully created. We are glad to have you on board!</p><p>You can now sign in and get started:</p><div class=\"cta\"><a href=\"{{app_url}}\">Go to the app</a></div><hr class=\"divider\"><p class=\"muted\">If you have any questions or need help getting started, feel free to reach out to us at <a href=\"mailto:{{support_email}}\" style=\"color:#ab2815\">{{support_email}}</a>.</p></div><div class=\"footer\"><p>You received this email because an account was created using your email address.<br>If you did not sign up, you can safely ignore this email.</p></div></div></div></body></html>",
    "bodyText": "Hi {{name}},\n\nYour account has been successfully created. Welcome aboard!\n\nSign in here:\n{{app_url}}\n\nIf you have questions, contact us at {{support_email}}."
  }
}$json$
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();

INSERT INTO _settings (name, value)
VALUES (
    'app',
    '{
  "appName": "Crabbase",
  "appUrl": "http://localhost:8080",
  "contactEmail": "admin@crabbase.local",
  "allowPublicUserRegistration": false
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();

INSERT INTO _settings (name, value)
VALUES (
    'mail',
    '{
  "senderName": "Crabbase Support",
  "senderAddress": "support@crabbase.local",
  "smtpHost": "",
  "smtpPort": 587,
  "smtpUsername": "",
  "smtpPassword": ""
}'
)
ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated = now();
