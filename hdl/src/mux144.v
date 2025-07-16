
`timescale 1ns/1ps

module mux144
(
  input clk, // Входной сигнал тактирования
  input reset, // Входной сигнал для асинхронный сброс
  input ptt, // Входной сигнал включения передачи. 0 активный
  input [2:0] band, // Входной сигнал от декодера диапазона
  output reg lna144, // Выходной сигнал включения LNA
  output reg pa144, // Выходной сигнал включения нижнего PA
  output reg a144, // Выходной сигнал включения предусилителя, реле, верхнего PA
  output reg lna430, // Выходной сигнал включения LNA
  output reg pa430, // Выходной сигнал включения нижнего PA
  output reg a430 // Выходной сигнал включения предусилителя, реле, верхнего PA
);

// Возможные состояния banddata
localparam WRONG_BAND 				= 3'b000;	// неправильный диапазон - всё отключить. старший бит 0
localparam RX144TX144 				= 3'b100;	// только 144
localparam RX430TX430 				= 3'b101;	// только 430
localparam RX430TX144 				= 3'b110;	// 430 приём, 144 передача
localparam RX144TX430 				= 3'b111;	// 144 приём, 430 передача

reg        [3:0] BandState;

// Выходы секвенсора
wire lna;
wire pa;
wire a;

// подключаем секвенсор
seq144 seq (
   .clk(clk),
   .reset(reset),
   .ptt(ptt),
   .lna144(lna),
   .pa144(pa),
   .a144(a)
);

// Отработка сброса 
always @(posedge clk)
  begin
    if(reset)
      BandState <= 0;
    else
      BandState <= band;
  end

//задание выходов
always @(posedge clk)
  begin
    if(reset)
      begin
        lna144 <= 1'b0;
        a144   <= 1'b0;  
        pa144  <= 1'b0;
        lna430 <= 1'b0;
        a430   <= 1'b0;  
        pa430  <= 1'b0;
      end
    else
      begin
        //передаем сигналы в соответствии с диапазоном 
        case(BandState)
          WRONG_BAND: 
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX144TX144:
            begin
		      lna144 <= lna;
			  a144   <= a;  
              pa144  <= pa;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX430TX430:
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= lna;
              a430   <= a;  
              pa430  <= pa;
            end
          RX430TX144:
            begin
		      lna144 <= 1'b0;
			  a144   <= a;  
              pa144  <= pa;
              lna430 <= 1'b1;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
          RX144TX430:
            begin
		      lna144 <= 1'b1;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= a;  
              pa430  <= pa;
            end
          default: // аналогично WRONG_BAND
            begin
		      lna144 <= 1'b0;
			  a144   <= 1'b0;  
              pa144  <= 1'b0;
              lna430 <= 1'b0;
              a430   <= 1'b0;  
              pa430  <= 1'b0;
            end
        endcase
      end
  end
  
endmodule