<?php
/**
 * Plugin Name: SatGate - Lightning Pay-to-Submit
 * Description: Simple integration for SatGate Lightning-gated forms.
 * Version: 1.0.0
 * Author: SatGate
 */

if (!defined('ABSPATH')) exit;

add_action('admin_menu', 'satgate_menu');
function satgate_menu() {
    add_options_page('SatGate Settings', 'SatGate', 'manage_options', 'satgate', 'satgate_settings_page');
}

function satgate_settings_page() {
    ?>
    <div class="wrap">
        <h1>SatGate Settings</h1>
        <form method="post" action="options.php">
            <?php
            settings_fields('satgate_settings');
            do_settings_sections('satgate');
            submit_button();
            ?>
        </form>
    </div>
    <?php
}

add_action('admin_init', 'satgate_settings_init');
function satgate_settings_init() {
    register_setting('satgate_settings', 'satgate_site_id');
    register_setting('satgate_settings', 'satgate_amount');

    add_settings_section('satgate_main', 'Main Settings', null, 'satgate');

    add_settings_field('satgate_site_id', 'Site ID', function() {
        $val = get_option('satgate_site_id');
        echo '<input type="text" name="satgate_site_id" value="' . esc_attr($val) . '" class="regular-text">';
    }, 'satgate', 'satgate_main');

    add_settings_field('satgate_amount', 'Default Amount (SATS)', function() {
        $val = get_option('satgate_amount', '100');
        echo '<input type="number" name="satgate_amount" value="' . esc_attr($val) . '">';
    }, 'satgate', 'satgate_main');
}

add_action('wp_footer', 'satgate_inject_script');
function satgate_inject_script() {
    $site_id = get_option('satgate_site_id');
    $amount = get_option('satgate_amount', '100');
    if (!$site_id) return;
    
    // Replace with your production URL
    echo '<script src="http://localhost:3000/widget.js" data-site-id="' . esc_attr($site_id) . '" data-amount="' . esc_attr($amount) . '"></script>';
}
