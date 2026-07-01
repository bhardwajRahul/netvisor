//! Organization-specific (Brevo/Stripe/target-IP) filters.
use super::*;

impl<T: Storable> StorableFilter<T> {
    // =========================================================================
    // Organization filters
    // =========================================================================

    /// Filter for organizations that haven't been synced to Brevo yet
    pub fn without_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NULL", col));
        self
    }

    /// Filter for organizations that have already been synced to Brevo
    pub fn with_brevo_company_id(mut self) -> Self {
        let col = self.qualify_column("brevo_company_id");
        self.conditions.push(format!("{} IS NOT NULL", col));
        self
    }

    /// Filter for organizations by Stripe customer ID
    pub fn stripe_customer_id(mut self, id: &str) -> Self {
        let col = self.qualify_column("stripe_customer_id");
        self.conditions
            .push(format!("{} = ${}", col, self.values.len() + 1));
        self.values.push(SqlValue::String(id.to_string()));
        self
    }
}
