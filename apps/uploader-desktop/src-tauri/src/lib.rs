//! Библиотечный таргет uploader-desktop: команды, БД очереди и воркеры
//! доступны интеграционным тестам (крейт integration-tests).
pub mod commands;
pub mod db;
pub mod worker;
