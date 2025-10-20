//! Type-safe SQL query builder
//!
//! Provides a fluent API for building SQL queries with automatic parameter binding
//! to prevent SQL injection attacks.

use super::value::DatabaseValue;

/// SQL comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// Equal to (=)
    Eq,
    /// Not equal to (<> or !=)
    Ne,
    /// Less than (<)
    Lt,
    /// Less than or equal (<=)
    Le,
    /// Greater than (>)
    Gt,
    /// Greater than or equal (>=)
    Ge,
    /// LIKE pattern matching
    Like,
    /// IN set membership
    In,
    /// IS NULL
    IsNull,
    /// IS NOT NULL
    IsNotNull,
}

impl Operator {
    fn as_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "!=",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
            Operator::Like => "LIKE",
            Operator::In => "IN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
        }
    }
}

/// WHERE clause condition
#[derive(Debug, Clone)]
pub struct Condition {
    column: String,
    operator: Operator,
    value: Option<DatabaseValue>,
}

/// JOIN types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    /// INNER JOIN
    Inner,
    /// LEFT JOIN
    Left,
    /// RIGHT JOIN
    Right,
    /// FULL OUTER JOIN
    Full,
}

impl JoinType {
    fn as_sql(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL OUTER JOIN",
        }
    }
}

/// JOIN clause
#[derive(Debug, Clone)]
pub struct Join {
    join_type: JoinType,
    table: String,
    on_condition: String,
}

/// ORDER BY direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

impl OrderDirection {
    fn as_sql(&self) -> &'static str {
        match self {
            OrderDirection::Asc => "ASC",
            OrderDirection::Desc => "DESC",
        }
    }
}

/// SELECT query builder
#[derive(Debug, Clone)]
pub struct SelectBuilder {
    table: String,
    columns: Vec<String>,
    joins: Vec<Join>,
    where_conditions: Vec<Condition>,
    where_logic: String, // "AND" or "OR"
    order_by: Vec<(String, OrderDirection)>,
    limit: Option<usize>,
    offset: Option<usize>,
    group_by: Vec<String>,
    #[allow(dead_code)] // Reserved for future HAVING clause support
    having_conditions: Vec<Condition>,
}

impl SelectBuilder {
    /// Create a new SELECT query builder
    ///
    /// # Example
    ///
    /// ```
    /// use rust_database_system::core::query_builder::SelectBuilder;
    ///
    /// let query = SelectBuilder::new("users")
    ///     .columns(&["id", "name", "email"])
    ///     .build();
    /// ```
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: vec!["*".to_string()],
            joins: Vec::new(),
            where_conditions: Vec::new(),
            where_logic: "AND".to_string(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            group_by: Vec::new(),
            having_conditions: Vec::new(),
        }
    }

    /// Select specific columns
    #[must_use]
    pub fn columns(mut self, columns: &[&str]) -> Self {
        self.columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Select all columns (*)
    #[must_use]
    pub fn all_columns(mut self) -> Self {
        self.columns = vec!["*".to_string()];
        self
    }

    /// Add a WHERE condition
    #[must_use]
    pub fn where_eq(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Eq,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column != value condition
    #[must_use]
    pub fn where_ne(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Ne,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column > value condition
    #[must_use]
    pub fn where_gt(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Gt,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column >= value condition
    #[must_use]
    pub fn where_ge(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Ge,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column < value condition
    #[must_use]
    pub fn where_lt(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Lt,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column <= value condition
    #[must_use]
    pub fn where_le(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Le,
            value: Some(value.into()),
        });
        self
    }

    /// Add a WHERE column LIKE pattern condition
    #[must_use]
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Like,
            value: Some(DatabaseValue::String(pattern.to_string())),
        });
        self
    }

    /// Add a WHERE column IS NULL condition
    #[must_use]
    pub fn where_null(mut self, column: &str) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::IsNull,
            value: None,
        });
        self
    }

    /// Add a WHERE column IS NOT NULL condition
    #[must_use]
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::IsNotNull,
            value: None,
        });
        self
    }

    /// Use OR logic for WHERE conditions instead of AND
    #[must_use]
    pub fn or_where(mut self) -> Self {
        self.where_logic = "OR".to_string();
        self
    }

    /// Add an INNER JOIN
    #[must_use]
    pub fn join(mut self, table: &str, on_condition: &str) -> Self {
        self.joins.push(Join {
            join_type: JoinType::Inner,
            table: table.to_string(),
            on_condition: on_condition.to_string(),
        });
        self
    }

    /// Add a LEFT JOIN
    #[must_use]
    pub fn left_join(mut self, table: &str, on_condition: &str) -> Self {
        self.joins.push(Join {
            join_type: JoinType::Left,
            table: table.to_string(),
            on_condition: on_condition.to_string(),
        });
        self
    }

    /// Add ORDER BY clause
    #[must_use]
    pub fn order_by(mut self, column: &str, direction: OrderDirection) -> Self {
        self.order_by.push((column.to_string(), direction));
        self
    }

    /// Add ORDER BY ASC
    #[must_use]
    pub fn order_by_asc(self, column: &str) -> Self {
        self.order_by(column, OrderDirection::Asc)
    }

    /// Add ORDER BY DESC
    #[must_use]
    pub fn order_by_desc(self, column: &str) -> Self {
        self.order_by(column, OrderDirection::Desc)
    }

    /// Add LIMIT clause
    #[must_use]
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add OFFSET clause
    #[must_use]
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Add GROUP BY clause
    #[must_use]
    pub fn group_by(mut self, columns: &[&str]) -> Self {
        self.group_by = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build the SQL query string
    pub fn build(&self) -> String {
        let mut sql = format!("SELECT {} FROM {}", self.columns.join(", "), self.table);

        // Add JOINs
        for join in &self.joins {
            sql.push_str(&format!(
                " {} {} ON {}",
                join.join_type.as_sql(),
                join.table,
                join.on_condition
            ));
        }

        // Add WHERE conditions
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let conditions: Vec<String> = self
                .where_conditions
                .iter()
                .enumerate()
                .map(|(i, cond)| {
                    let placeholder =
                        if matches!(cond.operator, Operator::IsNull | Operator::IsNotNull) {
                            format!("{} {}", cond.column, cond.operator.as_sql())
                        } else {
                            format!("{} {} ?", cond.column, cond.operator.as_sql())
                        };

                    if i == 0 {
                        placeholder
                    } else {
                        format!(" {} {}", self.where_logic, placeholder)
                    }
                })
                .collect();
            sql.push_str(&conditions.join(""));
        }

        // Add GROUP BY
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        // Add ORDER BY
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let order_clauses: Vec<String> = self
                .order_by
                .iter()
                .map(|(col, dir)| format!("{} {}", col, dir.as_sql()))
                .collect();
            sql.push_str(&order_clauses.join(", "));
        }

        // Add LIMIT
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Add OFFSET
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    /// Get the parameter values for parameterized queries
    pub fn params(&self) -> Vec<DatabaseValue> {
        self.where_conditions
            .iter()
            .filter_map(|cond| cond.value.clone())
            .collect()
    }
}

/// INSERT query builder
#[derive(Debug, Clone)]
pub struct InsertBuilder {
    table: String,
    columns: Vec<String>,
    values: Vec<DatabaseValue>,
}

impl InsertBuilder {
    /// Create a new INSERT query builder
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            columns: Vec::new(),
            values: Vec::new(),
        }
    }

    /// Add a column-value pair
    #[must_use]
    pub fn value(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.columns.push(column.to_string());
        self.values.push(value.into());
        self
    }

    /// Build the SQL query string
    pub fn build(&self) -> String {
        let placeholders: Vec<&str> = vec!["?"; self.values.len()];
        format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            self.columns.join(", "),
            placeholders.join(", ")
        )
    }

    /// Get the parameter values
    pub fn params(&self) -> Vec<DatabaseValue> {
        self.values.clone()
    }
}

/// UPDATE query builder
#[derive(Debug, Clone)]
pub struct UpdateBuilder {
    table: String,
    set_columns: Vec<String>,
    set_values: Vec<DatabaseValue>,
    where_conditions: Vec<Condition>,
    where_logic: String,
}

impl UpdateBuilder {
    /// Create a new UPDATE query builder
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            set_columns: Vec::new(),
            set_values: Vec::new(),
            where_conditions: Vec::new(),
            where_logic: "AND".to_string(),
        }
    }

    /// Set a column value
    #[must_use]
    pub fn set(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.set_columns.push(column.to_string());
        self.set_values.push(value.into());
        self
    }

    /// Add a WHERE condition
    #[must_use]
    pub fn where_eq(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Eq,
            value: Some(value.into()),
        });
        self
    }

    /// Build the SQL query string
    pub fn build(&self) -> String {
        let set_clauses: Vec<String> = self
            .set_columns
            .iter()
            .map(|col| format!("{} = ?", col))
            .collect();

        let mut sql = format!("UPDATE {} SET {}", self.table, set_clauses.join(", "));

        // Add WHERE conditions
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let conditions: Vec<String> = self
                .where_conditions
                .iter()
                .enumerate()
                .map(|(i, cond)| {
                    let placeholder = format!("{} {} ?", cond.column, cond.operator.as_sql());
                    if i == 0 {
                        placeholder
                    } else {
                        format!(" {} {}", self.where_logic, placeholder)
                    }
                })
                .collect();
            sql.push_str(&conditions.join(""));
        }

        sql
    }

    /// Get the parameter values (SET values followed by WHERE values)
    pub fn params(&self) -> Vec<DatabaseValue> {
        let mut params = self.set_values.clone();
        params.extend(
            self.where_conditions
                .iter()
                .filter_map(|cond| cond.value.clone()),
        );
        params
    }
}

/// DELETE query builder
#[derive(Debug, Clone)]
pub struct DeleteBuilder {
    table: String,
    where_conditions: Vec<Condition>,
    where_logic: String,
}

impl DeleteBuilder {
    /// Create a new DELETE query builder
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            where_conditions: Vec::new(),
            where_logic: "AND".to_string(),
        }
    }

    /// Add a WHERE condition
    #[must_use]
    pub fn where_eq(mut self, column: &str, value: impl Into<DatabaseValue>) -> Self {
        self.where_conditions.push(Condition {
            column: column.to_string(),
            operator: Operator::Eq,
            value: Some(value.into()),
        });
        self
    }

    /// Build the SQL query string
    pub fn build(&self) -> String {
        let mut sql = format!("DELETE FROM {}", self.table);

        // Add WHERE conditions
        if !self.where_conditions.is_empty() {
            sql.push_str(" WHERE ");
            let conditions: Vec<String> = self
                .where_conditions
                .iter()
                .enumerate()
                .map(|(i, cond)| {
                    let placeholder = format!("{} {} ?", cond.column, cond.operator.as_sql());
                    if i == 0 {
                        placeholder
                    } else {
                        format!(" {} {}", self.where_logic, placeholder)
                    }
                })
                .collect();
            sql.push_str(&conditions.join(""));
        }

        sql
    }

    /// Get the parameter values
    pub fn params(&self) -> Vec<DatabaseValue> {
        self.where_conditions
            .iter()
            .filter_map(|cond| cond.value.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_basic() {
        let query = SelectBuilder::new("users").build();
        assert_eq!(query, "SELECT * FROM users");
    }

    #[test]
    fn test_select_columns() {
        let query = SelectBuilder::new("users")
            .columns(&["id", "name", "email"])
            .build();
        assert_eq!(query, "SELECT id, name, email FROM users");
    }

    #[test]
    fn test_select_where() {
        let builder = SelectBuilder::new("users")
            .where_eq("id", 42)
            .where_eq("status", "active");

        assert_eq!(
            builder.build(),
            "SELECT * FROM users WHERE id = ? AND status = ?"
        );
        assert_eq!(builder.params().len(), 2);
    }

    #[test]
    fn test_select_order_limit() {
        let query = SelectBuilder::new("users")
            .order_by_desc("created_at")
            .limit(10)
            .build();

        assert_eq!(
            query,
            "SELECT * FROM users ORDER BY created_at DESC LIMIT 10"
        );
    }

    #[test]
    fn test_insert() {
        let builder = InsertBuilder::new("users")
            .value("name", "Alice")
            .value("email", "alice@example.com")
            .value("age", 30);

        assert_eq!(
            builder.build(),
            "INSERT INTO users (name, email, age) VALUES (?, ?, ?)"
        );
        assert_eq!(builder.params().len(), 3);
    }

    #[test]
    fn test_update() {
        let builder = UpdateBuilder::new("users")
            .set("name", "Bob")
            .set("age", 31)
            .where_eq("id", 1);

        assert_eq!(
            builder.build(),
            "UPDATE users SET name = ?, age = ? WHERE id = ?"
        );
        assert_eq!(builder.params().len(), 3);
    }

    #[test]
    fn test_delete() {
        let builder = DeleteBuilder::new("users").where_eq("id", 42);

        assert_eq!(builder.build(), "DELETE FROM users WHERE id = ?");
        assert_eq!(builder.params().len(), 1);
    }

    #[test]
    fn test_select_join() {
        let query = SelectBuilder::new("users")
            .columns(&["users.name", "orders.total"])
            .join("orders", "users.id = orders.user_id")
            .build();

        assert_eq!(
            query,
            "SELECT users.name, orders.total FROM users INNER JOIN orders ON users.id = orders.user_id"
        );
    }

    #[test]
    fn test_select_where_null() {
        let query = SelectBuilder::new("users").where_null("deleted_at").build();

        assert_eq!(query, "SELECT * FROM users WHERE deleted_at IS NULL");
    }

    #[test]
    fn test_select_complex() {
        let builder = SelectBuilder::new("users")
            .columns(&["id", "name", "email"])
            .where_eq("status", "active")
            .where_gt("age", 18)
            .order_by_asc("name")
            .limit(100)
            .offset(20);

        let query = builder.build();
        assert!(query.contains("SELECT id, name, email FROM users"));
        assert!(query.contains("status = ?") && query.contains("age > ?"));
        assert!(query.contains("ORDER BY name ASC"));
        assert!(query.contains("LIMIT 100"));
        assert!(query.contains("OFFSET 20"));
    }
}
