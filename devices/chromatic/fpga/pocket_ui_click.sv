// SPDX-License-Identifier: GPL-3.0-only
// Physical menu-button feedback in the Chromatic gClk domain (8.388608 MHz).
// No sample memory, MCU task, UART command, or new pin is required.
`timescale 1ns/1ps
module pocket_ui_click (
    input wire clk,
    input wire reset_n,
    input wire menu_open,
    input wire software_mute,
    input wire appear_off,
    input wire [7:0] buttons,
    input wire [15:0] game_left,
    input wire [15:0] game_right,
    output wire [15:0] left,
    output wire [15:0] right
);
    // Rounded up: 12 ms burst, at least 30 ms between accepted presses.
    localparam [16:0] CLICK_CYCLES = 17'd100664;
    localparam [17:0] RETRIGGER_CYCLES = 18'd251659;

    reg [7:0] previous_buttons;
    reg armed;
    reg [16:0] remaining;
    reg [17:0] cooldown;
    reg [10:0] phase;
    reg polarity;
    wire enabled = menu_open && !software_mute && !appear_off;
    wire pressed = |(buttons & ~previous_buttons);

    always @(posedge clk or negedge reset_n) begin
        if (!reset_n) begin
            previous_buttons <= 8'd0;
            armed <= 1'b0;
            remaining <= 17'd0;
            cooldown <= 18'd0;
            phase <= 11'd0;
            polarity <= 1'b0;
        end else begin
            // Track even while disabled; entering a menu with a held key,
            // unmuting, and reset release must not synthesize a new press.
            previous_buttons <= buttons;
            armed <= 1'b1;
            if (cooldown != 0) cooldown <= cooldown - 1'b1;

            if (!enabled) begin
                remaining <= 17'd0;
                phase <= 11'd0;
                polarity <= 1'b0;
            end else if (armed && pressed && cooldown == 0) begin
                remaining <= CLICK_CYCLES;
                // This accepted edge already counts as the first cycle.
                cooldown <= RETRIGGER_CYCLES - 1'b1;
                phase <= 11'd0;
                polarity <= 1'b0;
            end else if (remaining != 0) begin
                remaining <= remaining - 1'b1;
                phase <= phase + 1'b1;
                if (&phase) polarity <= ~polarity;
            end
        end
    end

    // 2048 Hz square wave, quantized linear decay. Peak magnitude 786/32768.
    // The enable gates suppress the added tone immediately, before a clock.
    wire signed [15:0] level = $signed({6'd0, remaining[16:7]});
    wire signed [15:0] tone = (reset_n && enabled && remaining != 0)
        ? (polarity ? -level : level) : 16'sd0;

    function automatic [15:0] mix_saturated;
        input [15:0] sample;
        input signed [15:0] click;
        reg signed [16:0] sum;
        begin
            sum = $signed({sample[15], sample}) + $signed({click[15], click});
            if (sum > 17'sd32767) mix_saturated = 16'h7fff;
            else if (sum < -17'sd32768) mix_saturated = 16'h8000;
            else mix_saturated = sum[15:0];
        end
    endfunction

    assign left = mix_saturated(game_left, tone);
    assign right = mix_saturated(game_right, tone);
endmodule
